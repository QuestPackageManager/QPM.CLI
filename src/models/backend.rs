use semver::Version;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[allow(non_snake_case)]
#[serde(rename_all = "camelCase")]
pub struct PackageVersion {
    pub id: String,
    pub version: Version,
}