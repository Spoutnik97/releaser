use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Package {
    pub path: String,
    #[serde(default)]
    #[serde(rename = "extraFiles")]
    pub extra_files: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Manifest {
    pub packages: Vec<Package>,
}
