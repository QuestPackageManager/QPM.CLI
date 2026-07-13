use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use qpm_package::models::package::DependencyId;
use semver::Version;

use crate::models::config::UserConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageIdPath(pub DependencyId);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageVersionPath(pub PackageIdPath, pub Version);

impl PackageIdPath {
    pub fn new(id: DependencyId) -> Self {
        Self(id)
    }

    pub fn version(self, version: Version) -> PackageVersionPath {
        PackageVersionPath(self, version)
    }

    pub fn versions_path(&self) -> PathBuf {
        let combine = UserConfig::read_combine().unwrap();
        let cache = combine.cache.as_ref().unwrap();
        cache.join(self.0.to_string())
    }
}

impl PackageVersionPath {
    pub fn new(id: DependencyId, version: Version) -> Self {
        Self(PackageIdPath::new(id), version)
    }

    /// Returns the base path for the package version.
    /// cache/{id}/{version}
    pub fn base_path(&self) -> PathBuf {
        self.versions_path().join(self.1.to_string())
    }

    /// Returns the path to the source files e.g headers for the package version.
    /// cache/{id}/{version}/src
    pub fn src_path(&self) -> PathBuf {
        self.base_path().join("src")
    }

    pub fn qpm_json_dir(&self) -> PathBuf {
        self.base_path()
    }
    pub fn qpkg_json_dir(&self) -> PathBuf {
        self.base_path()
    }

    /// Returns the path to the temporary files for the package version.
    /// cache/{id}/{version}/tmp
    pub fn tmp_path(&self) -> PathBuf {
        self.base_path().join("tmp")
    }

    /// Returns the path to the binaries for the package version.
    /// cache/{id}/{version}/lib
    pub fn binaries_path(&self) -> PathBuf {
        self.base_path().join("lib")
    }

    pub fn binary_path(&self, binary: &Path) -> PathBuf {
        self.binaries_path()
            .join(binary.file_name().expect("Binary file name"))
    }
}

impl Deref for PackageVersionPath {
    type Target = PackageIdPath;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
