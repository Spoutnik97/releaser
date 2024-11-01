pub use self::manager::{get_manifest, get_version_and_name, update_package};
pub use self::types::{Manifest, Package};
mod manager;
mod types;
