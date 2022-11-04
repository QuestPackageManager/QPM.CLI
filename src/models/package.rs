use std::{fs::File, path::Path};

use color_eyre::Result;
use itertools::Itertools;
use qpm_package::models::{
    dependency::{Dependency, SharedDependency, SharedPackageConfig},
    package::PackageConfig,
};
use semver::VersionReq;

use crate::{repository::Repository, resolver::dependency::resolve};

pub trait PackageConfigExtensions {
    fn read(dir: &Path) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn write(&self, dir: &Path) -> Result<()>;
}
pub trait SharedPackageConfigExtensions: Sized {
    fn resolve_from_package(
        config: PackageConfig,
        repository: &impl Repository,
    ) -> Result<(Self, Vec<SharedPackageConfig>)>;
}

impl PackageConfigExtensions for PackageConfig {
    fn read(dir: &Path) -> Result<Self> {
        let file = File::open(dir.with_file_name("qpm.json"))?;
        Ok(serde_json::from_reader(file)?)
    }

    fn write(&self, dir: &Path) -> Result<()> {
        let file = File::create(dir.with_file_name("qpm.json"))?;

        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}
impl PackageConfigExtensions for SharedPackageConfig {
    fn read(dir: &Path) -> Result<Self> {
        let file = File::open(dir.with_file_name("qpm.shared.json"))?;
        Ok(serde_json::from_reader(file)?)
    }

    fn write(&self, dir: &Path) -> Result<()> {
        let file = File::create(dir.with_file_name("qpm.shared.json"))?;

        serde_json::to_writer_pretty(file, self)?;
        Ok(())
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
                            additional_data: d.config.additional_data.clone(),
                        },
                        version: d.config.info.version.clone(),
                    })
                    .collect(),
            },
            resolved_deps,
        ))
    }
}
