use clap::Parser;
use colored::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Result, Value};
use std::io::Write;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
}; // Add this to your dependencies

// Add this helper function at the top level
fn log_section(title: &str) {
    println!("\n{}", "━".repeat(50).bright_black());
    println!("{}", title.bright_blue().bold());
    println!("{}", "━".repeat(50).bright_black());
}

fn log_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

fn log_info(message: &str) {
    println!("{} {}", "ℹ".blue(), message);
}

fn log_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Package {
    path: String,
    #[serde(default)]
    #[serde(rename = "extraFiles")]
    extra_files: Vec<String>,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Manifest {
    packages: Vec<Package>,
}

fn get_manifest() -> Result<Manifest> {
    let file_path = String::from("releaser-manifest.json");
    let manifest_raw =
        fs::read_to_string(file_path).expect("releaser-manifest.json file not found");
    let packages: Vec<Package> = serde_json::from_str(&manifest_raw)?;
    Ok(Manifest { packages })
}

fn has_dependency_changes(package: &Package, changed_packages: &HashMap<String, String>) -> bool {
    package
        .dependencies
        .iter()
        .any(|dep| changed_packages.contains_key(dep))
}

fn get_version_and_name(path: &str) -> Result<(String, String)> {
    let package_json_raw = fs::read_to_string(path.to_string() + "/package.json")
        .expect("Should have been able to read the file");

    let package_json: Value = serde_json::from_str(&package_json_raw)?;

    let version = package_json["version"].as_str().unwrap();
    let name = package_json["name"].as_str().unwrap();
    Ok((name.to_string(), version.to_string()))
}

fn get_latest_tag(name: &str, version: &str, environment: &str) -> Result<String> {
    let tag_prefix = format!("{}-v", name);

    // Get all tags for this package
    let output = std::process::Command::new("git")
        .args(&["tag", "-l", &format!("{}*", tag_prefix)])
        .output()
        .expect("Failed to execute git tag command");

    let tags = String::from_utf8_lossy(&output.stdout);

    // Filter and sort tags based on environment
    let latest_tag = tags
        .lines()
        .filter(|tag| {
            if environment == "production" {
                !tag.contains("-beta")
            } else {
                true // In non-production, consider all tags
            }
        })
        .max_by(|a, b| {
            // Custom comparison for semantic versioning
            let version_a = a.trim_start_matches(&tag_prefix);
            let version_b = b.trim_start_matches(&tag_prefix);
            semver_compare(version_a, version_b)
        });

    match latest_tag {
        Some(tag) => Ok(tag.to_string()),
        None => Ok(format!("{}-v{}", name, version)), // Return current version if no tags found
    }
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

// Helper function to compare semantic versions
fn semver_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.split('-').collect();
    let b_parts: Vec<&str> = b.split('-').collect();

    // Compare main version numbers first
    let a_version = a_parts[0].split('.').collect::<Vec<&str>>();
    let b_version = b_parts[0].split('.').collect::<Vec<&str>>();

    // Compare major.minor.patch
    for i in 0..3 {
        let a_num = a_version[i].parse::<u32>().unwrap_or(0);
        let b_num = b_version[i].parse::<u32>().unwrap_or(0);
        match a_num.cmp(&b_num) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    // If versions are equal, compare beta versions
    match (a_parts.get(1), b_parts.get(1)) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater, // Release is greater than beta
        (Some(_), None) => std::cmp::Ordering::Less,    // Beta is less than release
        (Some(a_beta), Some(b_beta)) => {
            // Compare beta version numbers if present
            let a_beta_num = a_beta
                .trim_start_matches("beta.")
                .parse::<u32>()
                .unwrap_or(0);
            let b_beta_num = b_beta
                .trim_start_matches("beta.")
                .parse::<u32>()
                .unwrap_or(0);
            a_beta_num.cmp(&b_beta_num)
        }
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
    extra_files: &Vec<String>,
    new_version: &str,
    dry_run: &DryRunConfig,
) {
    for extra_file in extra_files {
        let contents = fs::read_to_string(extra_file).expect("Failed to read file");

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
            fs::write(extra_file, new_contents).expect("Failed to write to file");
        } else {
            println!("Dry run: Would update version in file: {}", extra_file);
        }

        println!("Updated version in file: {}", extra_file);
    }
}

fn determine_semver_target(name: &str, version: &str, environment: &str) -> Semver {
    let last_tag = get_latest_tag(name, version, environment).unwrap();
    let interval = format!("{}..HEAD", last_tag);
    let git_log_output = std::process::Command::new("git")
        .args(&["log", &interval, "--oneline"])
        .output()
        .expect("Failed to execute git log command");

    let git_log_result = String::from_utf8_lossy(&git_log_output.stdout);

    let mut semver_target = Semver::Patch;
    for line in git_log_result.lines() {
        if line.contains("feat(") {
            semver_target = get_higher_semver(semver_target, Semver::Minor);
        }
        if line.contains("!feat(") || line.contains("!fix(") {
            return Semver::Major;
        }
    }
    semver_target
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The deployment environment (e.g., "production", "staging")
    #[arg(value_name = "ENVIRONMENT", default_value = "production")]
    environment: String,

    /// Perform a dry run without making any actual changes
    #[arg(long)]
    dry_run: bool,

    /// Create git tags for the current versions
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

    log_section("Release Process Started");

    if dry_run_config.is_dry_run {
        log_warning("Running in dry-run mode - No changes will be made");
    }
    log_info(&format!("Environment: {}", args.environment.bright_cyan()));

    let environment = &args.environment;
    let manifest: Manifest = get_manifest().unwrap();
    let mut changed_packages = HashMap::new();
    let mut pull_request_content = String::new();
    let mut name_to_version = HashMap::new();
    let mut tags_to_create = Vec::new();

    log_section("Analyzing Packages");

    for package in &manifest.packages {
        let (name, version) = get_version_and_name(&package.path).unwrap();
        let last_tag = get_latest_tag(&name, &version, &environment).unwrap();

        println!(
            "{} {} ({})",
            "📦".bright_cyan(),
            name.bright_white().bold(),
            package.path.bright_black()
        );
        println!("   Current version: {}", version.bright_yellow());
        println!("   Latest tag: {}", last_tag.bright_yellow());

        if args.tag {
            let tag = format!("{}-v{}", name, version);
            if !dry_run_config.is_dry_run {
                // Check if the tag already exists
                let tag_exists = std::process::Command::new("git")
                    .args(&["tag", "-l", &tag])
                    .output()
                    .map(|output| !output.stdout.is_empty())
                    .unwrap_or(false);

                if tag_exists {
                    println!("Tag {} already exists. Skipping tag creation.", tag);
                } else {
                    tags_to_create.push(tag.clone());
                    let tag_result = std::process::Command::new("git")
                        .args(&["tag", "-a", &tag, "-m", &tag])
                        .output();

                    match tag_result {
                        Ok(output) => {
                            if output.status.success() {
                                println!("Created new tag: {}", tag);
                            } else {
                                let error = String::from_utf8_lossy(&output.stderr);
                                eprintln!("Failed to create tag: {}. Error: {}", tag, error);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error executing git tag command: {}", e);
                        }
                    }
                }

                log_success(&format!("Created new tag: {}", tag));
            } else {
                log_info(&format!("Would create tag: {}", tag));
            }

            continue;
        }

        let output = std::process::Command::new("git")
            .args(&[
                "diff",
                "--name-only",
                &last_tag,
                "HEAD",
                "--",
                &package.path,
            ])
            .output()
            .expect("Failed to execute git diff command");

        let git_diff_result = String::from_utf8_lossy(&output.stdout);

        if git_diff_result.is_empty() {
            log_info("No changes detected - Skipping");

            continue;
        }

        let interval = last_tag + "..HEAD";
        let git_log_output = std::process::Command::new("git")
            .args(&["log", &interval, "--oneline", "--", &package.path])
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

            let current_changelog = fs::read_to_string(package.path.clone() + "/CHANGELOG.md").ok();
            let updated_changelog = update_changelog(
                current_changelog.as_deref(),
                &name,
                &changelog_body,
                &dry_run_config,
            )
            .expect("Changelog update failed");

            if !dry_run_config.is_dry_run {
                fs::write(
                    package.path.to_string() + "/CHANGELOG.md",
                    updated_changelog,
                )
                .expect("Failed to write updated CHANGELOG.md");
            }

            let filtered_changelog_body: String = changelog_body
                .lines()
                .filter(|line| !line.starts_with('#'))
                .collect::<Vec<&str>>()
                .join("\n");

            pull_request_content.push_str(format!("## {} - {}\n", name, new_version).as_str());
            pull_request_content.push_str(format!("{}\n\n", filtered_changelog_body).as_str());
        }

        update_package(&package.path, &new_version, &dry_run_config).unwrap();

        log_success(&format!(
            "Updated {} from {} to {}",
            name,
            version.bright_yellow(),
            new_version.bright_green()
        ));

        if !package.extra_files.is_empty() {
            increase_extra_files_version(&package.extra_files, &new_version, &dry_run_config);
        } else {
            println!("No extraFiles found for package {}", name);
        }

        changed_packages.insert(name.clone(), new_version.clone());
        name_to_version.insert(name.to_string(), new_version.to_string());
    }

    // Second pass: update packages and their dependencies
    for package in &manifest.packages {
        let (name, version) = get_version_and_name(&package.path).unwrap();
        let mut should_update = changed_packages.contains_key(&name);

        if !should_update {
            should_update = has_dependency_changes(package, &changed_packages);
        }

        if should_update {
            // Determine new version (consider both direct changes and dependency updates)
            let semver_target = if changed_packages.contains_key(&name) {
                determine_semver_target(&name, &version, &environment)
            } else {
                Semver::Patch // For dependency updates, use patch version
            };

            let new_version = increase_version(&version, semver_target, &environment);

            update_package(&package.path, &new_version, &dry_run_config).unwrap();

            if !package.extra_files.is_empty() {
                increase_extra_files_version(&package.extra_files, &new_version, &dry_run_config);
            }

            changed_packages.insert(name.clone(), new_version.clone());
        }
    }

    if args.tag {
        let tags_file_path = "tags_to_create.txt";
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(tags_file_path)
            .expect("Failed to open tags file");

        for tag in &tags_to_create {
            if let Err(e) = writeln!(file, "{}", tag) {
                eprintln!("Couldn't write to file: {}", e);
            }
        }

        log_section("Tag Creation Summary");
        log_success(&format!(
            "Created {} tags - List written to {}",
            tags_to_create.len(),
            "tags_to_create.txt".bright_cyan()
        ));
        return ();
    }

    log_section("Commit Changes");
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
        log_success("Created new commit with version bumps");
    } else {
        log_info("Would create git commit with version bumps");
    }

    if !dry_run_config.is_dry_run {
        fs::write("pull_request_content.md", pull_request_content).unwrap();
    }

    log_section("Summary");
    log_success(&format!(
        "Updated {} packages",
        changed_packages.len().to_string().bright_green()
    ));
    log_success("Pull request content written to pull_request_content.md");
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::{mock, predicate::*};
    use std::fs;
    use tempfile::NamedTempFile;

    mock! {
        FileSystem {
            fn read_to_string(&self, path: &str) -> std::io::Result<String>;
            fn write(&self, path: &str, contents: &str) -> std::io::Result<()>;
        }
    }

    mock! {
        GitCommand {
            fn diff(&self, last_tag: &str, path: &str) -> String;
            fn log(&self, interval: &str) -> String;
        }
    }

    fn setup_mocks() -> (MockFileSystem, MockGitCommand) {
        let mut fs = MockFileSystem::new();
        let git = MockGitCommand::new();

        fs.expect_read_to_string()
            .with(eq("releaser-manifest.json"))
            .returning(|_| {
                Ok(r#"{
                    "packages": [
                        {
                            "path": "package1",
                            "dependencies": ["package2"]
                        },
                        {
                            "path": "package2",
                            "dependencies": []
                        }
                    ]
                }"#
                .to_string())
            });

        (fs, git)
    }

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

            let extra_files = vec![file_path.clone()];

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

    #[test]
    fn test_dependency_update() {
        let (mut fs, mut git) = setup_mocks();

        // Mock package1 (no changes)
        fs.expect_read_to_string()
            .with(eq("package1/package.json"))
            .returning(|_| Ok(r#"{"name": "package1", "version": "1.0.0"}"#.to_string()));
        git.expect_diff()
            .with(eq("package1-v1.0.0"), eq("package1"))
            .returning(|_, _| String::new());

        // Mock package2 (with changes)
        fs.expect_read_to_string()
            .with(eq("package2/package.json"))
            .returning(|_| Ok(r#"{"name": "package2", "version": "1.0.0"}"#.to_string()));
        git.expect_diff()
            .with(eq("package2-v1.0.0"), eq("package2"))
            .returning(|_, _| "some_changed_file.js".to_string());
        git.expect_log()
            .with(eq("package2-v1.0.0..HEAD"))
            .returning(|_| "abcdef1 fix(package2): some bugfix".to_string());

        // Expect updates
        fs.expect_write()
            .with(
                eq("package2/package.json"),
                eq(r#"{"name": "package2", "version": "1.0.1"}"#),
            )
            .returning(|_, _| Ok(()));
        fs.expect_write()
            .with(
                eq("package1/package.json"),
                eq(r#"{"name": "package1", "version": "1.0.1"}"#),
            )
            .returning(|_, _| Ok(()));
    }

    #[test]
    fn test_get_latest_tag() {
        // Setup test environment
        let setup_git_tags = |tags: &[&str]| {
            // Clear existing tags
            let _ = std::process::Command::new("git")
                .args(&["tag", "-d"])
                .args(tags)
                .output();

            // Create new tags
            for tag in tags {
                let _ = std::process::Command::new("git")
                    .args(&["tag", tag])
                    .output();
            }
        };

        // Test case 1: Production environment with mixed tags
        let tags = &[
            "package-a-v1.0.0",
            "package-a-v1.0.1-beta",
            "package-a-v1.0.1",
            "package-a-v1.1.0-beta.1",
        ];
        setup_git_tags(tags);

        assert_eq!(
            get_latest_tag("package-a", "1.0.0", "production").unwrap(),
            "package-a-v1.0.1"
        );

        // Test case 2: Staging environment with beta tags
        assert_eq!(
            get_latest_tag("package-a", "1.0.0", "staging").unwrap(),
            "package-a-v1.1.0-beta.1"
        );

        // Test case 3: No tags exist
        let no_tags_package = "package-b";
        assert_eq!(
            get_latest_tag(no_tags_package, "1.0.0", "production").unwrap(),
            format!("{}-v1.0.0", no_tags_package)
        );

        // Cleanup
        let _ = std::process::Command::new("git")
            .args(&["tag", "-d"])
            .args(tags)
            .output();
    }

    #[test]
    fn test_semver_compare() {
        // Test regular versions
        assert!(matches!(
            semver_compare("1.0.0", "1.0.1"),
            std::cmp::Ordering::Less
        ));
        assert!(matches!(
            semver_compare("1.1.0", "1.0.1"),
            std::cmp::Ordering::Greater
        ));
        assert!(matches!(
            semver_compare("1.0.0", "1.0.0"),
            std::cmp::Ordering::Equal
        ));

        // Test beta versions
        assert!(matches!(
            semver_compare("1.0.0-beta", "1.0.0"),
            std::cmp::Ordering::Less
        ));
        assert!(matches!(
            semver_compare("1.0.0", "1.0.0-beta"),
            std::cmp::Ordering::Greater
        ));
        assert!(matches!(
            semver_compare("1.0.0-beta.1", "1.0.0-beta.2"),
            std::cmp::Ordering::Less
        ));
        assert!(matches!(
            semver_compare("1.0.0-beta.2", "1.0.0-beta.1"),
            std::cmp::Ordering::Greater
        ));

        // Test mixed scenarios
        assert!(matches!(
            semver_compare("1.0.1-beta", "1.0.0"),
            std::cmp::Ordering::Greater
        ));
        assert!(matches!(
            semver_compare("1.0.0-beta", "1.0.1"),
            std::cmp::Ordering::Less
        ));
    }
}
