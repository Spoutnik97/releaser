use clap::error::Result;

use crate::DryRunConfig;

pub struct Changelog {
    pub features: String,
    pub fixes: String,
    pub perf: String,
    pub breaking: String,
}

pub fn update_changelog(
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

pub fn get_new_changelog(name: &str, new_version: &str, changelog: Changelog) -> Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
