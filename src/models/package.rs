use std::{fs::File, io::BufReader, path::Path};

use color_eyre::{eyre::Context, Result};
use itertools::Itertools;
use qpm_package::models::{
    dependency::{Dependency, SharedDependency, SharedPackageConfig},
    package::PackageConfig,
};
use semver::VersionReq;

use crate::{repository::Repository, resolver::dependency::resolve, utils::json};

pub const PACKAGE_FILE_NAME: &str = "qpm.json";
pub const SHARED_PACKAGE_FILE_NAME: &str = "qpm.shared.json";

pub trait PackageConfigExtensions {
    fn exists<P: AsRef<Path>>(dir: P) -> bool;
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()>;
}
pub trait SharedPackageConfigExtensions: Sized {
    fn resolve_from_package(
        config: PackageConfig,
        repository: &impl Repository,
    ) -> Result<(Self, Vec<SharedPackageConfig>)>;
}

impl PackageConfigExtensions for PackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let file = File::open(dir.as_ref().join(PACKAGE_FILE_NAME))?;
        json::json_from_reader_fast(BufReader::new(file))
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let file = File::create(dir.as_ref().join(PACKAGE_FILE_NAME))?;

        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    fn exists<P: AsRef<Path>>(dir: P) -> bool {
        dir.as_ref().with_file_name(PACKAGE_FILE_NAME).exists()
    }
}
impl PackageConfigExtensions for SharedPackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let file = File::open(dir.as_ref().join(SHARED_PACKAGE_FILE_NAME))
            .context("Missing qpm.shared.json")?;
        json::json_from_reader_fast(BufReader::new(file))
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let file = File::create(dir.as_ref().join(SHARED_PACKAGE_FILE_NAME))?;

        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
    fn exists<P: AsRef<Path>>(dir: P) -> bool {
        dir.as_ref().join(SHARED_PACKAGE_FILE_NAME).exists()
    }
}

impl SharedPackageConfigExtensions for SharedPackageConfig {
    fn resolve_from_package(
        config: PackageConfig,
        repository: &impl Repository,
    ) -> Result<(Self, Vec<SharedPackageConfig>)> {
        let resolved_deps = resolve(&config, repository)?.collect_vec();

        Ok((
            SharedPackageConfig {
                config,
                restored_dependencies: resolved_deps
                    .iter()
                    .map(|d| SharedDependency {
                        dependency: Dependency {
                            id: d.config.info.id.clone(),
                            version_range: VersionReq::parse(&format!(
                                "={}",
                                d.config.info.version
                            ))
                            .expect("Unable to parse version"),
                            additional_data: d.config.info.additional_data.clone(),
                        },
                        version: d.config.info.version.clone(),
                    })
                    .collect(),
            },
            resolved_deps,
        ))
    }
}
