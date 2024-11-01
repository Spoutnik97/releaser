use regex::Regex;
use serde_json::Result;

use crate::semver_compare;

pub fn get_latest_tag(name: &str, version: &str, environment: &str) -> Result<String> {
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

pub fn format_commit_message(input: &str) -> String {
    let re = Regex::new(r"^[0-9a-f]+\s+\w+\(([^)]+)\):\s+(.+)$").unwrap();

    if let Some(captures) = re.captures(input) {
        let scope = captures.get(1).map_or("", |m| m.as_str());
        let message = captures.get(2).map_or("", |m| m.as_str());

        format!("**{}**: {}", scope, message)
    } else {
        input.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
