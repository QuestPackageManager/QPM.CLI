use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use qpm_package::models::{
    package::DependencyId,
    triplet::TripletId,
};
use semver::Version;

use crate::models::config::UserConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageIdPath(pub DependencyId);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageVersionPath(pub PackageIdPath, pub Version);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageTripletPath(pub PackageVersionPath, pub TripletId);

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

    pub fn triplet(self, triplet: TripletId) -> PackageTripletPath {
        PackageTripletPath(self, triplet)
    }

    /// Returns the base path for the package version, which includes the triplet.
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
}

impl PackageTripletPath {
    pub fn new(id: DependencyId, version: Version, triplet: TripletId) -> Self {
        Self(PackageVersionPath::new(id, version), triplet)
    }

    /// Returns the base path for the package triplet, which includes the triplet.
    /// cache/{id}/{version}/{triplet}
    pub fn triplet_path(&self) -> PathBuf {
        self.base_path().join(self.1.to_string())
    }

    /// Returns the base path for the package triplet, which includes the triplet.
    /// cache/{id}/{version}/{triplet}/lib
    pub fn binaries_path(&self) -> PathBuf {
        self.triplet_path().join("lib")
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

impl Deref for PackageTripletPath {
    type Target = PackageVersionPath;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
