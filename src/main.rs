use clap::Parser;
use regex::Regex;
use serde_json::{Result, Value};
use std::{collections::HashMap, fs};

fn get_manifest() -> Result<Value> {
    let file_path = String::from("releaser-manifest.json");
    let manifest_raw =
        fs::read_to_string(file_path).expect("releaser-manifest.json file not found");

    let manifest: Value = serde_json::from_str(&manifest_raw)?;
    Ok(manifest)
}

fn get_version_and_name(path: &str) -> Result<(String, String)> {
    let package_json_raw = fs::read_to_string(path.to_string() + "/package.json")
        .expect("Should have been able to read the file");

    let package_json: Value = serde_json::from_str(&package_json_raw)?;

    let version = package_json["version"].as_str().unwrap();
    let name = package_json["name"].as_str().unwrap();
    Ok((name.to_string(), version.to_string()))
}

fn get_latest_tag(name: &str, version: &str) -> Result<String> {
    let tag = format!("{}-v{}", name, version);
    Ok(tag)
}

enum Semver {
    Patch,
    Minor,
    Major,
}

fn get_higher_semver(current_semver: Semver, new_semver: Semver) -> Semver {
    match current_semver {
        Semver::Patch => match new_semver {
            Semver::Patch => Semver::Patch,
            Semver::Minor => Semver::Minor,
            Semver::Major => Semver::Major,
        },
        Semver::Minor => match new_semver {
            Semver::Patch => Semver::Minor,
            Semver::Minor => Semver::Minor,
            Semver::Major => Semver::Major,
        },
        Semver::Major => match new_semver {
            Semver::Patch => Semver::Major,
            Semver::Minor => Semver::Major,
            Semver::Major => Semver::Major,
        },
    }
}

fn increase_version(version: &str, semver: Semver, environment: &str) -> String {
    let captures = version.split("-").collect::<Vec<&str>>();
    let raw_version = captures[0];
    let is_beta = captures.len() > 1;

    let patch = raw_version.split(".").collect::<Vec<&str>>()[2]
        .parse::<u32>()
        .unwrap();
    let minor = raw_version.split(".").collect::<Vec<&str>>()[1]
        .parse::<u32>()
        .unwrap();
    let major = raw_version.split(".").collect::<Vec<&str>>()[0]
        .parse::<u32>()
        .unwrap();

    if environment == "production" {
        if is_beta {
            format!("{}.{}.{}", major, minor, patch)
        } else {
            match semver {
                Semver::Patch => format!("{}.{}.{}", major, minor, patch + 1),
                Semver::Minor => format!("{}.{}.0", major, minor + 1),
                Semver::Major => format!("{}.0.0", major + 1),
            }
        }
    } else {
        if is_beta {
            let beta_raw_version = captures[1].split(".").collect::<Vec<&str>>();
            let beta_version = if beta_raw_version.len() > 1 {
                beta_raw_version[1].parse::<u32>().unwrap()
            } else {
                0
            };

            match semver {
                Semver::Patch => format!(
                    "{}.{}.{}-beta.{}",
                    major,
                    minor,
                    patch + 1,
                    beta_version + 1
                ),
                Semver::Minor => format!("{}.{}.0-beta.{}", major, minor + 1, beta_version + 1),
                Semver::Major => format!("{}.0.0-beta.{}", major + 1, beta_version + 1),
            }
        } else {
            match semver {
                Semver::Patch => format!("{}.{}.{}-beta", major, minor, patch + 1),
                Semver::Minor => format!("{}.{}.0-beta", major, minor + 1),
                Semver::Major => format!("{}.0.0-beta", major + 1),
            }
        }
    }
}

fn get_new_changelog(name: &str, new_version: &str, changelog: Changelog) -> Result<String> {
    let mut new_changelog = String::new();
    new_changelog.push_str(format!("# {}", name).as_str());
    new_changelog.push_str("\n");
    new_changelog.push_str(format!("## Version {}", new_version).as_str());
    new_changelog.push_str("\n");
    if !changelog.features.is_empty() {
        new_changelog.push_str("### Features\n");
        new_changelog.push_str(&changelog.features);
        new_changelog.push_str("\n");
    }
    if !changelog.fixes.is_empty() {
        new_changelog.push_str("### Fixes\n");
        new_changelog.push_str(&changelog.fixes);
        new_changelog.push_str("\n");
    }
    if !changelog.perf.is_empty() {
        new_changelog.push_str("### Performance\n");
        new_changelog.push_str(&changelog.perf);
        new_changelog.push_str("\n");
    }

    Ok(new_changelog)
}

struct Changelog {
    features: String,
    fixes: String,
    perf: String,
    breaking: String,
}

fn update_package(package_path: &str, new_version: &str, dry_run: &DryRunConfig) -> Result<()> {
    if dry_run.is_dry_run {
        println!(
            "Dry run: Would update {} to version {}",
            package_path, new_version
        );
        return Ok(());
    }

    let package_json_path = package_path.to_string() + "/package.json";
    let package_json_raw =
        fs::read_to_string(&package_json_path).expect("Should have been able to read the file");

    let mut package_json: serde_json::Map<String, Value> =
        serde_json::from_str(&package_json_raw).expect("Should have been able to parse JSON");

    package_json.insert(
        "version".to_string(),
        Value::String(new_version.to_string()),
    );

    fs::write(
        &package_json_path,
        serde_json::to_string_pretty(&package_json).unwrap(),
    )
    .expect("Failed to write updated package.json");

    Ok(())
}

fn format_commit_message(input: &str) -> String {
    let re = Regex::new(r"^[0-9a-f]+\s+\w+\(([^)]+)\):\s+(.+)$").unwrap();

    if let Some(captures) = re.captures(input) {
        let scope = captures.get(1).map_or("", |m| m.as_str());
        let message = captures.get(2).map_or("", |m| m.as_str());

        format!("**{}**: {}", scope, message)
    } else {
        input.to_string()
    }
}

fn update_changelog(
    current_changelog: Option<&str>,
    name: &str,
    new_changelog_body: &str,
    dry_run: &DryRunConfig,
) -> Result<String> {
    if dry_run.is_dry_run {
        println!(
            "Dry run: Would update changelog for {} with new content",
            name
        );
        return Ok(new_changelog_body.to_string());
    }

    let mut updated_changelog = new_changelog_body.to_string();
    if let Some(current) = current_changelog {
        // Remove the package name from the existing changelog
        let existing_content = current.replace(&format!("# {}\n", name), "");
        // Append the existing content to the new changelog
        updated_changelog.push_str(&existing_content);
    } else {
        println!(
            "No existing changelog found for package {}. Creating new one...",
            name
        );
    }

    Ok(updated_changelog)
}

fn increase_extra_files_version(
    extra_files: &Vec<serde_json::Value>,
    new_version: &str,
    dry_run: &DryRunConfig,
) {
    for extra_file in extra_files {
        if let Some(file_path) = extra_file.as_str() {
            let contents = fs::read_to_string(file_path).expect("Failed to read file");

            let mut new_contents: String = contents
                .lines()
                .map(|line| {
                    if line.contains("// x-releaser-version") {
                        let parts: Vec<&str> = line.split("// x-releaser-version").collect();
                        let version_pattern =
                            regex::Regex::new(r"\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?").unwrap();

                        if let Some(version_match) = version_pattern.find(parts[0]) {
                            let old_version = version_match.as_str();
                            line.replace(old_version, new_version)
                        } else {
                            line.to_string()
                        }
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join("\n");

            // Preserve the original file's ending (with or without newline)
            if contents.ends_with('\n') {
                new_contents.push('\n');
            }

            if !dry_run.is_dry_run {
                fs::write(file_path, new_contents).expect("Failed to write to file");
            } else {
                println!("Dry run: Would update version in file: {}", file_path);
            }

            println!("Updated version in file: {}", file_path);
        } else {
            println!("Invalid file path in extra_files");
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "ENVIRONMENT", default_value = "production")]
    environment: String,

    #[arg(long)]
    dry_run: bool,

    #[arg(long)]
    tag: bool,
}

struct DryRunConfig {
    is_dry_run: bool,
}

fn main() {
    let args = Args::parse();

    let dry_run_config = DryRunConfig {
        is_dry_run: args.dry_run,
    };

    if dry_run_config.is_dry_run {
        println!("Running in dry-run mode. No changes will be made.");
    }

    let environment = &args.environment;
    let manifest: Value = get_manifest().unwrap();

    let manifest_array = manifest.as_array().unwrap();

    let mut pull_request_content = String::new();

    let mut name_to_version = HashMap::new();

    for package in manifest_array {
        if let Some(package_path) = package["path"].as_str() {
            let (name, version) = get_version_and_name(package_path).unwrap();
            let last_tag = get_latest_tag(&name, &version).unwrap();

            println!(
                "{}, -> {}: {} => tag: {}",
                package_path, name, version, last_tag
            );

            if args.tag {
                let tag = format!("{}-v{}", name, version);
                if !dry_run_config.is_dry_run {
                    std::process::Command::new("git")
                        .args(&["tag", "-a", &tag, "-m", &tag])
                        .output()
                        .expect("Failed to execute git tag command");

                    println!("Created new tag: {}", tag);
                } else {
                    println!("Dry run: would create tag {}", tag);
                }

                continue;
            }

            let output = std::process::Command::new("git")
                .args(&["diff", "--name-only", &last_tag, "HEAD", "--", package_path])
                .output()
                .expect("Failed to execute git diff command");

            let git_diff_result = String::from_utf8_lossy(&output.stdout);

            if git_diff_result.is_empty() {
                println!("No changes detected. Skipping...");
                continue;
            }

            let interval = last_tag + "..HEAD";
            let git_log_output = std::process::Command::new("git")
                .args(&["log", &interval, "--oneline", "--", package_path])
                .output()
                .expect("Failed to execute git log command");

            let git_log_result = String::from_utf8_lossy(&git_log_output.stdout);

            let mut changelog = Changelog {
                features: String::new(),
                fixes: String::new(),
                perf: String::new(),
                breaking: String::new(),
            };

            let mut semver_target: Semver = Semver::Patch;
            for line in git_log_result.lines() {
                let commit_message = format_commit_message(line);
                if line.contains("feat(") {
                    changelog.features.push_str(&commit_message);
                    changelog.features.push_str("\n");
                    semver_target = get_higher_semver(semver_target, Semver::Minor);
                }
                if line.contains("fix(") {
                    changelog.fixes.push_str(&commit_message);
                    changelog.fixes.push_str("\n");
                    semver_target = get_higher_semver(semver_target, Semver::Patch);
                }
                if line.contains("perf(") {
                    changelog.perf.push_str(&commit_message);
                    changelog.perf.push_str("\n");
                    semver_target = get_higher_semver(semver_target, Semver::Patch);
                }
                if line.contains("!feat(") || line.contains("!fix(") {
                    changelog.breaking.push_str(&commit_message);
                    changelog.breaking.push_str("\n");
                    semver_target = get_higher_semver(semver_target, Semver::Major);
                }
            }

            let new_version = increase_version(&version, semver_target, &environment);
            let new_changelog = get_new_changelog(&name, &new_version, changelog);

            if new_changelog.is_ok() {
                let changelog_body = new_changelog.unwrap();

                let current_changelog =
                    fs::read_to_string(package_path.to_string() + "/CHANGELOG.md").ok();
                let updated_changelog = update_changelog(
                    current_changelog.as_deref(),
                    &name,
                    &changelog_body,
                    &dry_run_config,
                )
                .expect("Changelog update failed");

                if !dry_run_config.is_dry_run {
                    fs::write(
                        package_path.to_string() + "/CHANGELOG.md",
                        updated_changelog,
                    )
                    .expect("Failed to write updated CHANGELOG.md");
                }

                pull_request_content.push_str(format!("### {} - {}\n", name, new_version).as_str());
                pull_request_content.push_str(format!("{}\n\n", changelog_body).as_str());
            }

            update_package(package_path, &new_version, &dry_run_config).unwrap();

            println!(
                "Updated package.json of {} to version {}",
                name, new_version
            );

            let extra_files = package["extraFiles"].as_array().unwrap();
            increase_extra_files_version(&extra_files.to_vec(), &new_version, &dry_run_config);

            name_to_version.insert(name.to_string(), new_version.to_string());
        }
    }

    if args.tag {
        return ();
    }
    if !dry_run_config.is_dry_run {
        std::process::Command::new("git")
            .args(&["add", "."])
            .output()
            .expect("Failed to execute git add command");
        let mut commit_message = String::new();
        commit_message.push_str("chore(release): bump packages");
        for (name, version) in name_to_version {
            commit_message.push_str(&format!("- {}: {}", name, version));
        }

        std::process::Command::new("git")
            .args(&["commit", "-m", &commit_message])
            .output()
            .expect("Failed to execute git commit command");
        println!("Created new commit: {}", commit_message);
    } else {
        println!("Dry run: Would execute git commands");
    }
    fs::write("pull_request_content.md", pull_request_content).unwrap();
    println!("Pull request content written to pull_request_content.md");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_increase_version() {
        assert_eq!(
            increase_version("1.2.3", Semver::Patch, "production"),
            "1.2.4"
        );
        assert_eq!(
            increase_version("1.2.3", Semver::Minor, "production"),
            "1.3.0"
        );
        assert_eq!(
            increase_version("1.2.3", Semver::Major, "production"),
            "2.0.0"
        );
        assert_eq!(
            increase_version("1.2.3", Semver::Patch, "staging"),
            "1.2.4-beta"
        );
        assert_eq!(
            increase_version("1.2.3", Semver::Minor, "staging"),
            "1.3.0-beta"
        );
        assert_eq!(
            increase_version("1.2.3", Semver::Major, "staging"),
            "2.0.0-beta"
        );
        assert_eq!(
            increase_version("1.2.3-beta", Semver::Major, "production"),
            "1.2.3"
        );
        assert_eq!(
            increase_version("1.2.3-beta.1", Semver::Patch, "production"),
            "1.2.3"
        );
        assert_eq!(
            increase_version("1.2.3-beta.1", Semver::Minor, "production"),
            "1.2.3"
        );
        assert_eq!(
            increase_version("1.2.3-beta.1", Semver::Major, "production"),
            "1.2.3"
        );
        assert_eq!(
            increase_version("1.2.3-beta", Semver::Minor, "staging"),
            "1.3.0-beta.1"
        );
        assert_eq!(
            increase_version("1.2.3", Semver::Minor, "staging"),
            "1.3.0-beta"
        );
    }

    #[test]
    fn test_format_commit_message() {
        assert_eq!(
            format_commit_message(
                "195eabb15 fix(prediction): fix all the prediction refetch. The UX was flickering"
            ),
            "**prediction**: fix all the prediction refetch. The UX was flickering"
        );
        assert_eq!(
            format_commit_message("081b0001c fix(quote request line): the process step product lines name and price were not updated"), "**quote request line**: the process step product lines name and price were not updated");
        assert_eq!(
            format_commit_message(
                "f0c7441f1 feat(quote request line): fix React Hook Form context in sync with API"
            ),
            "**quote request line**: fix React Hook Form context in sync with API"
        );
    }

    #[test]
    fn test_increase_extra_files_version() {
        let test_cases = vec![
            ("1.2.3", "1.2.4"),
            ("1.2.3", "1.3.0"),
            ("1.2.3", "2.0.0"),
            ("1.2.3-beta", "1.2.3"),
            ("1.2.3-beta.1", "1.2.3"),
            ("1.2.3", "1.2.4-beta"),
            ("1.2.3", "1.3.0-beta"),
            ("1.2.3", "2.0.0-beta"),
            ("1.2.3-beta", "1.2.4-beta.1"),
            ("1.2.3-beta.1", "1.2.4-beta.2"),
        ];

        for (old_version, new_version) in test_cases {
            let temp_file = NamedTempFile::new().unwrap();
            let file_path = temp_file.path().to_str().unwrap().to_string();

            let content = format!(
                "const VERSION = '{}'; // x-releaser-version\nOther content\n",
                old_version
            );
            fs::write(&file_path, content).unwrap();

            let extra_files = vec![serde_json::Value::String(file_path.clone())];

            increase_extra_files_version(
                &extra_files,
                new_version,
                &DryRunConfig { is_dry_run: false },
            );

            let updated_content = fs::read_to_string(&file_path).unwrap();

            let expected_line = format!("const VERSION = '{}'; // x-releaser-version", new_version);
            assert!(
                updated_content.contains(&expected_line),
                "Failed to update from {} to {}",
                old_version,
                new_version
            );
            assert!(updated_content.contains("Other content"));
        }
    }

    #[test]
    fn test_update_changelog() {
        let name = "test-package";
        let new_version = "1.2.3";
        let dry_run_config = DryRunConfig { is_dry_run: false };

        // Test case 1: New changelog, no existing content
        let changelog = Changelog {
            features: "- New feature 1\n- New feature 2\n".to_string(),
            fixes: "- Bug fix 1\n".to_string(),
            perf: "- Performance improvement 1\n".to_string(),
            breaking: "".to_string(),
        };
        let new_changelog_body = get_new_changelog(name, new_version, changelog).unwrap();
        let result = update_changelog(None, name, &new_changelog_body, &dry_run_config).unwrap();

        assert!(result.starts_with(&format!("# {}\n## Version {}", name, new_version)));
        assert!(result.contains("### Features"));
        assert!(result.contains("### Fixes"));
        assert!(result.contains("### Performance"));

        // Test case 2: Updating existing changelog
        let existing_changelog = format!(
            "# {}\n## Version 1.1.0\n### Features\n- Old feature\n",
            name
        );
        let result = update_changelog(
            Some(&existing_changelog),
            name,
            &new_changelog_body,
            &dry_run_config,
        )
        .unwrap();

        assert!(result.starts_with(&format!("# {}\n## Version {}", name, new_version)));
        assert!(result.contains("- New feature 1"));
        assert!(result.contains("## Version 1.1.0"));
        assert!(result.contains("- Old feature"));

        // Ensure new version comes before old version
        assert!(
            result.find("## Version 1.2.3").unwrap() < result.find("## Version 1.1.0").unwrap()
        );

        // Test case 3: Dry run
        let dry_run_config = DryRunConfig { is_dry_run: true };
        let result = update_changelog(
            Some(&existing_changelog),
            name,
            &new_changelog_body,
            &dry_run_config,
        )
        .unwrap();

        assert_eq!(result, new_changelog_body);
    }
}
