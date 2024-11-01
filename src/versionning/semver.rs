pub enum Semver {
    Patch,
    Minor,
    Major,
}
pub fn increase_version(version: &str, semver: Semver, environment: &str) -> String {
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

pub fn semver_compare(a: &str, b: &str) -> std::cmp::Ordering {
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
