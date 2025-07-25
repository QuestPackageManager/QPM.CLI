use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
    path::Path,
};

use color_eyre::{Result, Section, eyre::Context, owo_colors::OwoColorize};
use itertools::Itertools;
use qpm_package::models::{
    package::{DependencyId, PackageConfig, QPM_JSON},
    shared_package::{
        QPM_SHARED_JSON, SharedPackageConfig, SharedTriplet, SharedTripletDependencyInfo,
    },
    triplet::{PackageTriplet, PackageTripletDependency, TripletId},
};
use qpm_qmod::models::mod_json::{ModDependency, ModJson};
use semver::VersionReq;
use serde::de;

use crate::{
    repository::Repository,
    resolver::dependency::{ResolvedDependency, resolve},
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
    fn resolve_from_package(
        config: PackageConfig,
        repository: &impl Repository,
    ) -> Result<(Self, HashMap<TripletId, Vec<ResolvedDependency>>)>;

    fn to_mod_json(self, repo: &impl Repository) -> ModJson;

    fn try_write_toolchain(&self, repo: &impl Repository) -> Result<()>;

    fn get_restored_triplet(&self) -> &SharedTriplet;
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
        let path = dir.as_ref().join(QPM_SHARED_JSON);
        let file = File::open(&path)
            .with_context(|| format!("{path:?} not found"))
            .suggestion(format!("Try running {}", "qpm restore".blue()))?;

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

struct DependencyBundle<'a> {
    triplet: &'a TripletId,

    dep_config: PackageConfig,
    dep_triplet: PackageTriplet,

    shared_restored_triplet: &'a SharedTripletDependencyInfo,
    restored_triplet: &'a PackageTripletDependency,
}

impl SharedPackageConfigExtensions for SharedPackageConfig {
    /// Resolve dependencies from the package config and repository
    /// Returns a tuple of the SharedPackageConfig and a map of triplet IDs to resolved
    fn resolve_from_package(
        config: PackageConfig,
        repository: &impl Repository,
    ) -> Result<(Self, HashMap<TripletId, Vec<ResolvedDependency>>)> {
        // for each triplet, resolve the dependencies
        let triplet_dependencies: HashMap<TripletId, _> = config
            .triplets
            .specific_triplets
            .iter()
            .map(|(triplet_id, _triplet)| -> color_eyre::Result<_> {
                let resolved = resolve(&config, repository, triplet_id)?.collect_vec();

                Ok((triplet_id.clone(), resolved))
            })
            .try_collect()?;

        let locked_triplet = triplet_dependencies
            .iter()
            .map(|(package_triplet, dependencies)| {
                // For each dependency, get the package config and triplet settings
                let restored_dependencies = dependencies
                    .iter()
                    .map(|resolved_dep| {
                        let shared_triplet_dependency_info = SharedTripletDependencyInfo {
                            restored_version: resolved_dep.0.version.clone(),
                            restored_triplet: resolved_dep.1.clone(),
                        };
                        (resolved_dep.0.id.clone(), shared_triplet_dependency_info)
                    })
                    .collect();
                let shared_triplet = SharedTriplet {
                    restored_dependencies,
                };

                (package_triplet.clone(), shared_triplet)
            })
            .collect();

        let shared_package_config = SharedPackageConfig {
            config,
            restored_triplet: Default::default(),
            locked_triplet,
        };
        Ok((shared_package_config, triplet_dependencies))
    }

    fn to_mod_json(self, repo: &impl Repository) -> ModJson {
        //        Self {
        //     id: dep.id,
        //     version_range: dep.version_range,
        //     mod_link: dep.additional_data.mod_link,
        // }

        // List of dependencies we are directly referencing in qpm.json
        let package_triplet = self
            .config
            .triplets
            .get_triplet_settings(&self.restored_triplet)
            .expect("Triplet should exist");

        let direct_dependencies: HashMap<DependencyId, _> = package_triplet
            .dependencies
            .iter()
            .filter_map(|(dep_id, dep_triplet)| {
                // get the restored dependency info
                let shared_dep_triplet = self
                    .get_restored_triplet()
                    .restored_dependencies
                    .get(dep_id)?;

                // get the package config for the dependency
                let dep_package = repo
                    .get_package(&dep_id, &shared_dep_triplet.restored_version)
                    .expect("Failed to get package")
                    .expect("Package should exist in repository");

                // get the triplet settings for the dependency
                let dep_package_triplet = dep_package
                    .triplets
                    .get_triplet_settings(&shared_dep_triplet.restored_triplet)
                    .expect("Triplet should exist in package");

                let result = DependencyBundle {
                    triplet: &shared_dep_triplet.restored_triplet,

                    shared_restored_triplet: shared_dep_triplet,
                    restored_triplet: dep_triplet,

                    // grabbed from repo
                    dep_config: dep_package,
                    dep_triplet: dep_package_triplet,
                };
                Some((dep_id.clone(), result))
            })
            .collect();

        // downloadable mods links n stuff
        // mods that are header-only but provide qmods can be added as deps
        // Must be directly referenced in qpm.json
        let mods: Vec<ModDependency> = direct_dependencies
            .iter()
            // Removes any dependency without a qmod link
            .filter(|(dep_package, result)| result.dep_triplet.qmod_url.is_some())
            .map(|(dep_config, result)| ModDependency {
                version_range: result.restored_triplet.version_range.clone(),
                id: result.dep_config.id.0.clone(),
                mod_link: result.dep_triplet.qmod_url.clone(),
                required: result.restored_triplet.qmod_required,
            })
            .collect();

        ModJson {
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
        }
    }

    fn try_write_toolchain(&self, repo: &impl Repository) -> Result<()> {
        let Some(toolchain_path) = self.config.toolchain_out.as_ref() else {
            return Ok(());
        };

        toolchain::write_toolchain_file(self, repo, toolchain_path)?;

        Ok(())
    }

    fn get_restored_triplet(&self) -> &SharedTriplet {
        self.locked_triplet
            .get(&self.restored_triplet)
            .expect("Restored triplet should exist in locked triplet map")
    }
}
