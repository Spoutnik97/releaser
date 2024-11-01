use std::fs;

use crate::DryRunConfig;

pub fn increase_extra_files_version(
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    use crate::DryRunConfig;

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
}
