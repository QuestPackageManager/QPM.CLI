use semver::{Version, VersionReq};

use serde::{Deserialize, Serialize};

use super::{extra::AdditionalPackageMetadata, package::PackageConfig};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub id: String,
    #[serde(deserialize_with = "cursed_semver_parser::deserialize")]
    pub version_range: VersionReq,
    pub additional_data: AdditionalPackageMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SharedDependency {
    pub dependency: Dependency,
    pub version: Version,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[allow(non_snake_case)]
#[serde(rename_all = "camelCase")]
pub struct SharedPackageConfig {
    /// The packageconfig that is stored in qpm.json
    pub config: PackageConfig,
    /// The dependencies as given by self.config.resolve()
    pub restored_dependencies: Vec<SharedDependency>,
}
