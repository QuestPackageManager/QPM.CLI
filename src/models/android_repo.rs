use serde::{Deserialize, Serialize};

// https://dl.google.com/android/repository/repository2-3.xml

/// https://android.googlesource.com/platform/tools/base/+/studio-master-dev/repository/src/main/java/com/android/repository/impl/generated/v1/RepositoryType.java
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AndroidRepositoryManifest {
    pub license: Vec<LicenseType>,
    pub remote_package: Vec<RemotePackage>,
}

/// https://android.googlesource.com/platform/tools/base/+/studio-master-dev/repository/src/main/java/com/android/repository/impl/generated/v1/ChannelType.java
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ChannelType {
    #[serde(rename = "$value")]
    pub value: String,

    #[serde(alias = "ID")]
    pub id: String,
}

/// https://android.googlesource.com/platform/tools/base/+/studio-master-dev/repository/src/main/java/com/android/repository/impl/generated/v1/LicenseType.java
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LicenseType {
    #[serde(rename = "$value")]
    pub value: String,

    #[serde(alias = "ID")]
    pub id: String,

    #[serde(rename = "type")]
    pub typ: String,
}

/// https://android.googlesource.com/platform/tools/base/+/studio-master-dev/repository/src/main/java/com/android/repository/impl/generated/v1/RemotePackage.java
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RemotePackage {
    pub path: String,
    pub archives: ArchivesType,
    pub revision: RevisionType,
    #[serde(rename = "display-name")]
    pub display_name: String,
    #[serde(rename = "uses-license")]
    pub uses_license: Option<LicenseRefType>,
    #[serde(rename = "channelRef")]
    pub channel: Option<ChannelRefType>,
}

/// https://android.googlesource.com/platform/tools/base/+/studio-master-dev/repository/src/main/java/com/android/repository/impl/generated/v1/ChannelType.java
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ChannelRefType {
    #[serde(rename = "ref")]
    pub channel: String,
}

/// https://android.googlesource.com/platform/tools/base/+/studio-master-dev/repository/src/main/java/com/android/repository/impl/generated/v1/ChannelType.java
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LicenseRefType {
    #[serde(rename = "ref")]
    pub license: String,
}

/// https://android.googlesource.com/platform/tools/base/+/studio-master-dev/repository/src/main/java/com/android/repository/impl/generated/v1/ArchivesType.java
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ArchivesType {
    pub archive: Vec<Archive>,
}

/// https://android.googlesource.com/platform/tools/base/+/studio-master-dev/repository/src/main/java/com/android/repository/impl/generated/v1/ArchiveType.java
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Archive {
    #[serde(rename = "host-os")]
    pub host_os: Option<String>,
    #[serde(rename = "host-arch")]
    pub host_arch: Option<String>,
    pub complete: CompleteType,
}

/// https://android.googlesource.com/platform/tools/base/+/studio-master-dev/repository/src/main/java/com/android/repository/impl/generated/v1/CompleteType.java
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CompleteType {
    pub size: Option<usize>,
    pub checksum: String,
    pub url: String,
}

/// https://android.googlesource.com/platform/tools/base/+/studio-master-dev/repository/src/main/java/com/android/repository/impl/generated/v1/RevisionType.java
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RevisionType {
    pub major: Option<u64>,
    pub minor: Option<u64>,
    pub micro: Option<u64>,
    pub preview: Option<u64>,
}
