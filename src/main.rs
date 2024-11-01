use clap::Parser;
use colored::*;
use std::io::Write;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
};
mod cli;
use cli::Args;
mod logging;
use logging::*;
mod git;
use git::*;
mod versionning;
use versionning::*;
mod package;
use package::*;
mod changelog;
use changelog::*;
mod utils;
use utils::*;

fn has_dependency_changes(package: &Package, changed_packages: &HashMap<String, String>) -> bool {
    package
        .dependencies
        .iter()
        .any(|dep| changed_packages.contains_key(dep))
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

struct DryRunConfig {
    is_dry_run: bool,
}

fn process_tag_creation(
    name: &str,
    version: &str,
    dry_run_config: &DryRunConfig,
    tags_to_create: &mut Vec<String>,
) {
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
}

fn commit_changes(
    dry_run_config: &DryRunConfig,
    name_to_version: &HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

fn write_tags_file(tags_to_create: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let tags_file_path = "tags_to_create.txt";
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(tags_file_path)
        .expect("Failed to open tags file");

    for tag in tags_to_create {
        if let Err(e) = writeln!(file, "{}", tag) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }
    Ok(())
}

fn process_package_changes(
    package: &Package,
    environment: &str,
    dry_run_config: &DryRunConfig,
    changed_packages: &mut HashMap<String, String>,
    name_to_version: &mut HashMap<String, String>,
    pull_request_content: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
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

        ()
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
    Ok(())
}

fn process_dependencies(
    packages: &[Package],
    environment: &str,
    dry_run_config: &DryRunConfig,
    changed_packages: &mut HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    for package in packages {
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
    Ok(())
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

    let manifest: Manifest = get_manifest().unwrap();
    let mut changed_packages = HashMap::new();
    let mut pull_request_content = String::new();
    let mut name_to_version = HashMap::new();
    let mut tags_to_create = Vec::new();

    log_section("Analyzing Packages");

    for package in &manifest.packages {
        if args.tag {
            let (name, version) = get_version_and_name(&package.path).unwrap();
            process_tag_creation(&name, &version, &dry_run_config, &mut tags_to_create);
            continue;
        }

        if let Err(e) = process_package_changes(
            package,
            &args.environment,
            &dry_run_config,
            &mut changed_packages,
            &mut name_to_version,
            &mut pull_request_content,
        ) {
            eprintln!("Error processing package: {}", e);
            std::process::exit(1);
        }
    }

    if args.tag {
        if let Err(e) = write_tags_file(&tags_to_create) {
            eprintln!("Error writing tags file: {}", e);
            std::process::exit(1);
        }
        log_section("Tag Creation Summary");
        log_success(&format!(
            "Created {} tags - List written to {}",
            tags_to_create.len(),
            "tags_to_create.txt".bright_cyan()
        ));
        return;
    }

    if let Err(e) = process_dependencies(
        &manifest.packages,
        &args.environment,
        &dry_run_config,
        &mut changed_packages,
    ) {
        eprintln!("Error processing dependencies: {}", e);
        std::process::exit(1);
    }

    log_section("Commit Changes");
    if let Err(e) = commit_changes(&dry_run_config, &name_to_version) {
        eprintln!("Error committing changes: {}", e);
        std::process::exit(1);
    }

    if !dry_run_config.is_dry_run {
        if let Err(e) = fs::write("pull_request_content.md", &pull_request_content) {
            eprintln!("Error writing pull request content: {}", e);
            std::process::exit(1);
        }
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
    use mockall::{mock, predicate::*};

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
}
