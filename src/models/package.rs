use std::path::PathBuf;

use semver::Version;
use serde::{Deserialize, Serialize};

use super::{dependency::Dependency, extra::AdditionalPackageMetadata};

/// qpm.json
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[allow(non_snake_case)]
#[serde(rename_all = "camelCase")]
pub struct PackageConfig {
    pub shared_dir: PathBuf,
    pub dependencies_dir: PathBuf,
    pub info: PackageMetadata,
    pub dependencies: Vec<Dependency>,
    pub additional_data: AdditionalPackageMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageMetadata {
    pub name: String,
    pub id: String,
    pub version: Version,
    pub url: Option<String>,
    pub additional_data: AdditionalPackageMetadata,
}
