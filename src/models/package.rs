use std::{collections::HashSet, fs::File, io::BufReader, path::Path};

use color_eyre::{eyre::Context, Result};
use itertools::Itertools;
use qpm_package::{
    extensions::package_metadata::PackageMetadataExtensions,
    models::{
        dependency::{Dependency, SharedDependency, SharedPackageConfig},
        package::PackageConfig,
    },
};
use qpm_qmod::models::mod_json::{ModDependency, ModJson};
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

    fn to_mod_json(self) -> ModJson;
}

impl PackageConfigExtensions for PackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let path = dir.as_ref().join(PACKAGE_FILE_NAME);
        let file = File::open(&path).with_context(|| format!("{path:?} does not exist"))?;
        json::json_from_reader_fast(BufReader::new(file))
            .with_context(|| format!("Unable to read PackageConfig at {path:?}"))
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
}
impl PackageConfigExtensions for SharedPackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let path = dir.as_ref().join(SHARED_PACKAGE_FILE_NAME);
        let file = File::open(&path).with_context(|| format!("{path:?} not found"))?;
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

    fn to_mod_json(self) -> ModJson {
        //        Self {
        //     id: dep.id,
        //     version_range: dep.version_range,
        //     mod_link: dep.additional_data.mod_link,
        // }

        let local_deps = &self.config.dependencies;

        // Only bundle mods that are not specifically excluded in qpm.json or if they're not header-only
        let restored_deps: Vec<_> = self
            .restored_dependencies
            .iter()
            .filter(|dep| {
                let local_dep_opt = local_deps
                    .iter()
                    .find(|local_dep| local_dep.id == dep.dependency.id);

                if let Some(local_dep) = local_dep_opt {
                    // if force included/excluded, return early
                    if let Some(force_included) = local_dep.additional_data.include_qmod {
                        return force_included;
                    }
                }

                // or if header only is false
                dep.dependency.additional_data.mod_link.is_some()
                    || !dep.dependency.additional_data.headers_only.unwrap_or(false)
            })
            .collect();

        // List of dependencies we are directly referencing in qpm.json
        let direct_dependencies: HashSet<String> = self
            .config
            .dependencies
            .iter()
            .map(|f| f.id.clone())
            .collect();

        // downloadable mods links n stuff
        // mods that are header-only but provide qmods can be added as deps
        // Must be directly referenced in qpm.json
        let mods: Vec<ModDependency> = local_deps
            .iter()
            // Removes any dependency without a qmod link
            .filter_map(|dep| {
                let shared_dep = restored_deps.iter().find(|d| d.dependency.id == dep.id)?;
                if shared_dep.dependency.additional_data.mod_link.is_some() {
                    return Some((shared_dep, dep));
                }

                None
            })
            .map(|(shared_dep, dep)| ModDependency {
                version_range: dep.version_range.clone(),
                id: dep.id.clone(),
                mod_link: shared_dep.dependency.additional_data.mod_link.clone(),
            })
            .collect();

        // The rest of the mods to handle are not qmods, they are .so or .a mods
        // actual direct lib deps
        let libs: Vec<String> = self
            .restored_dependencies
            .iter()
            // We could just query the bmbf core mods list on GH?
            // https://github.com/BMBF/resources/blob/master/com.beatgames.beatsaber/core-mods.json
            // but really the only lib that never is copied over is the modloader, the rest is either a downloaded qmod or just a copied lib
            // even core mods should technically be added via download
            .filter(|lib| {
                // find the actual dependency for the include qmod value
                let local_dep_opt = local_deps
                    .iter()
                    .find(|local_dep| local_dep.id == lib.dependency.id);

                // if set, use it later

                let include_qmod = local_dep_opt
                    .and_then(|local_dep| local_dep.additional_data.include_qmod.as_ref());

                // Must be directly referenced in qpm.json
                direct_dependencies.contains(&lib.dependency.id) &&

                // keep if header only is false, or if not defined
                !lib.dependency.additional_data.headers_only.unwrap_or(false) &&

                // Modloader should never be included
                lib.dependency.id != "modloader" &&

                // don't include static deps
                !lib.dependency.additional_data.static_linking.unwrap_or(false) &&

                // it's marked to be included, defaults to including ( same as dependencies with qmods )
                include_qmod.copied().unwrap_or(true) &&

                // Only keep libs that aren't downloadable
                !mods.iter().any(|dep| lib.dependency.id == dep.id)
            })
            .map(|dep| dep.get_so_name().to_str().unwrap().to_string())
            .collect();

        ModJson {
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
            // TODO: Change
            late_mod_files: vec![self.config.info.get_so_name().to_str().unwrap().to_string()],
            library_files: libs,
            ..Default::default()
        }
    }
}
