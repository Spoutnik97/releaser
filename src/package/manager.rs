use crate::DryRunConfig;
use serde_json::{Result, Value};
use std::fs;

use super::{Manifest, Package};

pub fn get_manifest() -> Result<Manifest> {
    let file_path = String::from("releaser-manifest.json");
    let manifest_raw =
        fs::read_to_string(file_path).expect("releaser-manifest.json file not found");
    let packages: Vec<Package> = serde_json::from_str(&manifest_raw)?;
    Ok(Manifest { packages })
}

pub fn update_package(package_path: &str, new_version: &str, dry_run: &DryRunConfig) -> Result<()> {
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

pub fn get_version_and_name(path: &str) -> Result<(String, String)> {
    let package_json_raw = fs::read_to_string(path.to_string() + "/package.json")
        .expect("Should have been able to read the file");

    let package_json: Value = serde_json::from_str(&package_json_raw)?;

    let version = package_json["version"].as_str().unwrap();
    let name = package_json["name"].as_str().unwrap();
    Ok((name.to_string(), version.to_string()))
}
