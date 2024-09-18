use regex::Regex;
use serde_json::{Result, Value};
use std::env;
use std::fs;

fn get_manifest(environment: &str) -> Result<Value> {
    if environment != "production" {
        let file_path = String::from("releaser-manifest.json");
        let manifest_raw =
            fs::read_to_string(file_path).expect("Should have been able to read the file");

        let manifest: Value = serde_json::from_str(&manifest_raw)?;
        Ok(manifest)
    } else {
        let file_path = String::from("releaser-manifest.production.json");
        let manifest_raw =
            fs::read_to_string(file_path).expect("Should have been able to read the file");

        let manifest: Value = serde_json::from_str(&manifest_raw)?;
        Ok(manifest)
    }
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
}

fn update_package(package_path: &str, new_version: &str) -> Result<()> {
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

fn update_changelog(package_path: &str, name: &str, changelog_body: &str) -> Result<()> {
    let current_changelog = fs::read_to_string(package_path.to_string() + "/CHANGELOG.md");
    if current_changelog.is_ok() {
        changelog_body.to_string().push_str(
            &current_changelog
                .unwrap()
                .replace(format!("# {}", name).as_str(), ""),
        );
        fs::write(package_path.to_string() + "/CHANGELOG.md", changelog_body)
            .expect("Failed to write updated CHANGELOG.md");
    } else {
        println!(
            "No changelog found for package {}. Creating new one...",
            package_path
        );
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let environment = &args[1];

    let manifest: Value = get_manifest(environment).unwrap();

    let manifest_array = manifest.as_array().unwrap();

    let mut pull_request_content = String::new();

    for key in manifest_array {
        if let Some(package_path) = key.as_str() {
            let (name, version) = get_version_and_name(package_path).unwrap();
            let last_tag = get_latest_tag(&name, &version).unwrap();

            println!("{}, -> {}: {} => {}", package_path, name, version, last_tag);

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
            }

            let new_version = increase_version(&version, semver_target, &environment);
            let new_changelog = get_new_changelog(&name, &new_version, changelog);

            if new_changelog.is_ok() {
                let changelog_body = new_changelog.unwrap();

                update_changelog(&package_path, &name, &changelog_body)
                    .expect("Changelog update failed");

                pull_request_content.push_str(format!("### {} - {}\n", name, new_version).as_str());
                pull_request_content.push_str(format!("{}\n\n", changelog_body).as_str());
            }

            update_package(package_path, &new_version).unwrap();

            println!(
                "Updated package.json of {} to version {}",
                name, new_version
            );

            std::process::Command::new("git")
                .args(&["add", "."])
                .output()
                .expect("Failed to execute git add command");

            let commit_message = format!("chore(release): bump {} to v{}", name, new_version);
            std::process::Command::new("git")
                .args(&["commit", "-m", &commit_message])
                .output()
                .expect("Failed to execute git commit command");

            let tag = format!("{}-v{}", name, new_version);
            std::process::Command::new("git")
                .args(&["tag", "-a", &tag, "-m", &commit_message])
                .output()
                .expect("Failed to execute git tag command");

            println!("Created new tag: {}", tag);
        }
    }
    fs::write("pull_request_content.md", pull_request_content).unwrap();
    println!("Pull request content written to pull_request_content.md");
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
