use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use color_eyre::{eyre::Context, owo_colors::OwoColorize, Result, Section};
use itertools::Itertools;
use qpm_package::{
    extensions::package_metadata::PackageMetadataExtensions,
    models::{
        dependency::{Dependency, SharedDependency, SharedPackageConfig},
        package::{PackageConfig, PackageDependency},
    },
};
use qpm_qmod::models::mod_json::{ModDependency, ModJson};
use semver::VersionReq;

use crate::{repository::Repository, resolver::dependency::resolve, utils::json};

use super::toolchain;

pub const PACKAGE_FILE_NAME: &str = "qpm.json";
pub const SHARED_PACKAGE_FILE_NAME: &str = "qpm.shared.json";

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
    fn resolve_from_package(
        config: PackageConfig,
        repository: &impl Repository,
    ) -> Result<(Self, Vec<SharedPackageConfig>)>;

    fn to_mod_json(self, repo: &impl Repository) -> color_eyre::Result<ModJson>;

    fn try_write_toolchain(&self, repo: &impl Repository) -> Result<()>;
}

impl PackageConfigExtensions for PackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let path = dir.as_ref().join(PACKAGE_FILE_NAME);
        let file = File::open(&path).with_context(|| format!("{path:?} does not exist"))?;
        let res = json::json_from_reader_fast::<_, Self>(BufReader::new(file))
            .with_context(|| format!("Unable to read PackageConfig at {path:?}"))?;
        res.validate()?;

        Ok(res)
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let path = dir.as_ref().join(PACKAGE_FILE_NAME);
        let file = File::create(&path).with_context(|| format!("{path:?} cannot be written"))?;

        serde_json::to_writer_pretty(file, self)
            .with_context(|| format!("Unable to write PackageConfig at {path:?}"))?;
        Ok(())
    }

    fn exists<P: AsRef<Path>>(dir: P) -> bool {
        dir.as_ref().with_file_name(PACKAGE_FILE_NAME).exists()
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

        if self.version.major != default.version.major {
            eprintln!(
                "Warning: using outdate qpm schema. Current {} Latest: {:?}",
                self.version, default.version
            );
        }

        Ok(())
    }
}
impl PackageConfigExtensions for SharedPackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let path = dir.as_ref().join(SHARED_PACKAGE_FILE_NAME);
        let file = File::open(&path)
            .with_context(|| format!("{path:?} not found"))
            .suggestion(format!("Try running {}", "qpm restore".blue()))?;

        json::json_from_reader_fast(BufReader::new(file))
            .with_context(|| format!("Unable to read SharedPackageConfig at {path:?}"))
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let path = dir.as_ref().join(SHARED_PACKAGE_FILE_NAME);
        let file = File::create(&path).with_context(|| format!("{path:?} cannot be written"))?;

        serde_json::to_writer_pretty(file, self)
            .with_context(|| format!("Unable to write PackageConfig at {path:?}"))?;
        Ok(())
    }
    fn exists<P: AsRef<Path>>(dir: P) -> bool {
        dir.as_ref().join(SHARED_PACKAGE_FILE_NAME).exists()
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

    fn to_mod_json(self, repo: &impl Repository) -> color_eyre::Result<ModJson> {
        //        Self {
        //     id: dep.id,
        //     version_range: dep.version_range,
        //     mod_link: dep.additional_data.mod_link,
        // }

        // List of dependencies we are directly referencing in qpm.json
        let direct_dependencies: HashMap<String, &PackageDependency> = self
            .config
            .dependencies
            .iter()
            .map(|f| (f.id.clone(), f))
            .collect();

        let restored_dependencies_inflated: HashMap<String, SharedPackageConfig> = self
            .restored_dependencies
            .into_iter()
            .map(|shared_dep| -> color_eyre::Result<_> {
                let dep_package =
                    repo.get_package_checked(&shared_dep.dependency.id, &shared_dep.version)?;
                Ok((shared_dep.dependency.id.clone(), dep_package))
            })
            .try_collect()?;

        // Only bundle mods that are not specifically excluded in qpm.json or if they're not header-only
        let restored_deps: Vec<_> = restored_dependencies_inflated
            .values()
            .filter(|dep_package| {
                if let Some(local_dep) = direct_dependencies.get(&dep_package.config.info.id) {
                    // if force included/excluded, return early
                    if let Some(force_included) = local_dep.additional_data.include_qmod {
                        return force_included;
                    }
                }

                // if a qmod, we need to depend on it
                dep_package.config.info.additional_data.mod_link.is_some()
                // if not header only, we link to it and bundle it later
                    || !dep_package
                        .config
                        .info
                        .additional_data
                        .headers_only
                        .unwrap_or(false)
            })
            .collect();

        // downloadable mods links n stuff
        // mods that are header-only but provide qmods can be added as deps
        // Must be directly referenced in qpm.json
        let mods: Vec<ModDependency> = direct_dependencies
            .values()
            // Removes any dependency without a qmod link
            .filter_map(|dep| {
                let shared_dep = restored_dependencies_inflated.get(&dep.id)?;
                if shared_dep.config.info.additional_data.mod_link.is_some() {
                    let mod_dependency = ModDependency {
                        version_range: dep.version_range.clone(),
                        id: dep.id.clone(),
                        mod_link: shared_dep.config.info.additional_data.mod_link.clone(),
                        required: dep.additional_data.required,
                    };
                    return Some(mod_dependency);
                }

                None
            })
            .sorted_by(|a, b| a.id.cmp(&b.id))
            .collect();

        // The rest of the mods to handle are not qmods, they are .so or .a mods
        // actual direct lib deps
        let libs: Vec<String> = restored_deps
            .iter()
            // We could just query the bmbf core mods list on GH?
            // https://github.com/BMBF/resources/blob/master/com.beatgames.beatsaber/core-mods.json
            // but really the only lib that never is copied over is the modloader, the rest is either a downloaded qmod or just a copied lib
            // even core mods should technically be added via download
            .filter(|lib| {
                // if set, use it later

                let include_qmod = direct_dependencies
                    .get(&lib.config.info.id)
                    .and_then(|dep| dep.additional_data.include_qmod);

                // Must be directly referenced in qpm.json
                direct_dependencies.contains_key(&lib.config.info.id) &&

                // keep if header only is false, or if not defined
                !lib.config.info.additional_data.headers_only.unwrap_or(false) &&

                // Modloader should never be included
                lib.config.info.id != "modloader" &&

                // don't include static deps
                !lib.config.info.additional_data.static_linking.unwrap_or(false) &&

                // it's marked to be included, defaults to including ( same as dependencies with qmods )
                include_qmod.unwrap_or(true) &&

                // Only keep libs that aren't downloadable
                !mods.iter().any(|dep| lib.config.info.id == dep.id)
            })
            .map(|dep| dep.config.info.get_so_name2().to_str().unwrap().to_string())
            .sorted()
            .collect();

        let json = ModJson {
            name: self.config.info.name.clone(),
            id: self.config.info.id.clone(),
            porter: None,
            version: self.config.info.version.to_string(),
            package_id: None,
            package_version: None,
            description: None,
            cover_image: None,
            is_library: None,
            dependencies: mods,
            late_mod_files: vec![self
                .config
                .info
                .get_so_name2()
                .to_str()
                .unwrap()
                .to_string()],
            library_files: libs,
            ..Default::default()
        };
        Ok(json)
    }

    fn try_write_toolchain(&self, repo: &impl Repository) -> Result<()> {
        let Some(toolchain_path) = self.config.info.additional_data.toolchain_out.as_ref() else {
            return Ok(());
        };

        toolchain::write_toolchain_file(self, repo, toolchain_path)?;

        Ok(())
    }
}
