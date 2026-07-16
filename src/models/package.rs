use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use color_eyre::{
    Result, Section,
    eyre::{Context, ContextCompat},
    owo_colors::OwoColorize,
};
use itertools::Itertools;
use qpm_package::models::{
    package::{DependencyId, PackageConfig, PackageDependency, QPM_JSON, QmodDependencyMode},
    shared_package::{QPM_SHARED_JSON, SharedPackageConfig},
};
use qpm_qmod::models::mod_json::{ModDependency, ModJson};
use semver::VersionReq;

use crate::{
    repository::{Repository, file::FileRepository},
    utils::json,
};

use super::{
    schemas::{SchemaLinks, WithSchema},
    toolchain,
};

pub trait PackageConfigExtensions {
    fn exists<P: AsRef<Path>>(dir: P) -> bool;
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()>;
    fn run_if_version(
        &self,
        req: &VersionReq,
        func: impl FnOnce() -> color_eyre::Result<()>,
    ) -> color_eyre::Result<()>;
    fn matches_version(&self, req: &VersionReq) -> bool;

    fn validate(&self) -> color_eyre::Result<()>;
}
pub trait SharedPackageConfigExtensions: Sized {
    fn to_mod_json(self, repo: &impl Repository) -> Result<ModJson>;

    fn try_write_toolchain(&self, repo: &impl Repository, file_repo: &FileRepository)
    -> Result<()>;

    fn get_env(&self) -> HashMap<String, String>;
}

impl PackageConfigExtensions for PackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let path = dir.as_ref().join(QPM_JSON);
        let file = File::open(&path).with_context(|| format!("{path:?} does not exist"))?;
        let res = json::json_from_reader_fast::<_, Self>(BufReader::new(file))
            .with_context(|| format!("Unable to read PackageConfig at {path:?}"))?;
        res.validate()?;

        Ok(res)
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let path = dir.as_ref().join(QPM_JSON);
        let file = File::create(&path).with_context(|| format!("{path:?} cannot be written"))?;

        serde_json::to_writer_pretty(
            file,
            &WithSchema {
                schema: SchemaLinks::PACKAGE_CONFIG,
                value: self,
            },
        )
        .with_context(|| format!("Unable to write PackageConfig at {path:?}"))?;
        Ok(())
    }

    fn exists<P: AsRef<Path>>(dir: P) -> bool {
        dir.as_ref().with_file_name(QPM_JSON).exists()
    }

    fn run_if_version(
        &self,
        req: &VersionReq,
        func: impl FnOnce() -> color_eyre::Result<()>,
    ) -> color_eyre::Result<()> {
        if req.matches(&self.version) {
            return func();
        }

        Ok(())
    }
    fn matches_version(&self, req: &VersionReq) -> bool {
        req.matches(&self.version)
    }

    fn validate(&self) -> color_eyre::Result<()> {
        let default = Self::default();

        if self.config_version.major != default.config_version.major {
            eprintln!(
                "Warning: using outdate qpm schema. Current {} Latest: {:?}",
                self.config_version, default.config_version
            );
        }

        Ok(())
    }
}
impl PackageConfigExtensions for SharedPackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let path = dir.as_ref().join(QPM_SHARED_JSON);
        let file = File::open(&path)
            .with_context(|| format!("{path:?} not found"))
            .suggestion(format!("Try running {}", "qpm2 restore".blue()))?;

        json::json_from_reader_fast(BufReader::new(file))
            .with_context(|| format!("Unable to read SharedPackageConfig at {path:?}"))
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let path = dir.as_ref().join(QPM_SHARED_JSON);
        let file = File::create(&path).with_context(|| format!("{path:?} cannot be written"))?;

        serde_json::to_writer_pretty(
            file,
            &WithSchema {
                schema: SchemaLinks::SHARED_PACKAGE_CONFIG,
                value: self,
            },
        )
        .with_context(|| format!("Unable to write PackageConfig at {path:?}"))?;
        Ok(())
    }
    fn exists<P: AsRef<Path>>(dir: P) -> bool {
        dir.as_ref().join(QPM_SHARED_JSON).exists()
    }

    fn run_if_version(
        &self,
        req: &VersionReq,
        func: impl FnOnce() -> color_eyre::Result<()>,
    ) -> color_eyre::Result<()> {
        self.config.run_if_version(req, func)
    }

    fn matches_version(&self, req: &VersionReq) -> bool {
        self.config.matches_version(req)
    }

    fn validate(&self) -> color_eyre::Result<()> {
        self.config.validate()
    }
}

/// Stores information about a dependency bundle
/// This includes the package config and triplet settings for the dependency
struct DependencyBundle<'a> {
    /// Package of the dependency, as resolved from the repository
    restored_config: PackageConfig,

    /// The dependency as specified in the root package
    dependency: &'a PackageDependency,
}

impl SharedPackageConfigExtensions for SharedPackageConfig {
    fn to_mod_json(self, repo: &impl Repository) -> Result<ModJson> {
        // Map of directly referenced dependencies
        let direct_dependencies: HashMap<DependencyId, _> = self
            .config
            .dependencies
            .iter()
            .filter_map(|(dep_id, dependency)| {
                // get the restored dependency info
                let shared_dep = self.restored_dependencies.get(dep_id)?;
                Some((dep_id, dependency, shared_dep))
            })
            .map(
                |(dep_id, dependency, shared_dep)| -> Result<(DependencyId, DependencyBundle)> {
                    // get the package config for the dependency
                    let dep_package = repo
                        .get_package(dep_id, &shared_dep.restored_version)?
                        .with_context(|| format!("Package {dep_id} should exist in repository"))?;

                    let result = DependencyBundle {
                        dependency,
                        restored_config: dep_package.config,
                    };
                    Ok((dep_id.clone(), result))
                },
            )
            .collect::<Result<_>>()?;

        // downloadable mods links n stuff
        // mods that are header-only but provide qmods can be added as deps
        // Must be directly referenced in qpm.json
        let mods: Vec<ModDependency> = direct_dependencies
            .values()
            // only on the qmod dependencies that aren't disabled
            .filter(|t| t.dependency.qmod != Some(QmodDependencyMode::None))
            // Removes any dependency without a qmod link
            .filter(|result| result.restored_config.qmod.download_url.is_some())
            .map(|result| ModDependency {
                version_range: result.dependency.version_range.clone(),
                id: result.restored_config.id.0.clone(),
                mod_link: result.restored_config.qmod.download_url.clone(),
                required: Some(result.dependency.qmod == Some(QmodDependencyMode::Required)),
            })
            .collect();

        Ok(ModJson {
            name: self.config.id.0.clone(),
            id: self.config.id.0.clone(),
            porter: None,
            version: self.config.version.to_string(),
            package_id: None,
            package_version: None,
            description: None,
            cover_image: None,
            is_library: None,
            dependencies: mods,
            late_mod_files: vec![],
            library_files: vec![],
            ..Default::default()
        })
    }

    fn try_write_toolchain(
        &self,
        repo: &impl Repository,
        file_repo: &FileRepository,
    ) -> Result<()> {
        let Some(toolchain_path) = self.config.workspace.toolchain_out.as_ref() else {
            return Ok(());
        };

        toolchain::write_toolchain_file(self, repo, toolchain_path, file_repo)?;

        Ok(())
    }

    fn get_env(&self) -> HashMap<String, String> {
        let dep_env = self
            .restored_dependencies
            .values()
            .map(|dep| &dep.restored_env)
            .collect_vec();

        // ensure no key collisions
        let mut flattened_map: HashMap<String, String> = HashMap::with_capacity(dep_env.len());
        for env in dep_env {
            for (key, value) in env {
                if flattened_map.contains_key(key) {
                    eprintln!(
                        "Warning: Environment variable {key} is defined multiple times, using the last value."
                    );
                }
                flattened_map.insert(key.clone(), value.clone());
            }
        }

        // we allow local environment variables to override the ones in the shared package
        flattened_map.extend(self.env.clone());

        flattened_map
    }
}
