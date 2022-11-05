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
    fn check<P: AsRef<Path>>(dir: P) -> bool;
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
        let file = File::open(dir.as_ref().with_file_name("qpm.json"))?;
        Ok(serde_json::from_reader(file)?)
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let file = File::create(dir.as_ref().with_file_name("qpm.json"))?;

        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    fn check<P: AsRef<Path>>(dir: P) -> bool {
        dir.as_ref().with_file_name("qpm.json").exists()
    }
}
impl PackageConfigExtensions for SharedPackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let file = File::open(dir.as_ref().with_file_name("qpm.shared.json"))?;
        Ok(serde_json::from_reader(file)?)
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let file = File::create(dir.as_ref().with_file_name("qpm.shared.json"))?;

        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
    fn check<P: AsRef<Path>>(dir: P) -> bool {
        dir.as_ref().with_file_name("qpm.shared.json").exists()
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
