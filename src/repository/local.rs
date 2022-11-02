use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use owo_colors::OwoColorize;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::Entry, HashMap},
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use remove_dir_all::remove_dir_all;

use qpm_package::models::{
    backend::PackageVersion, dependency::SharedPackageConfig, package::PackageConfig,
};

use crate::{
    models::{
        config::{get_combine_config, UserConfig},
        package::PackageConfigExtensions,
        package_metadata::PackageMetadataExtensions,
    },
    utils::fs::copy_things,
};

use super::Repository;

// TODO: Somehow make a global singleton of sorts/cached instance to share across places
// like resolver
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FileRepository {
    pub artifacts: HashMap<String, HashMap<Version, SharedPackageConfig>>,
    config: UserConfig,
}

impl FileRepository {
    pub fn get_artifacts_from_id(
        &self,
        id: &str,
    ) -> Option<&HashMap<Version, SharedPackageConfig>> {
        self.artifacts.get(id)
    }

    pub fn get_artifact(&self, id: &str, version: &Version) -> Option<&SharedPackageConfig> {
        match self.artifacts.get(id) {
            Some(artifacts) => artifacts.get(version),
            None => None,
        }
    }

    /// for adding to cache from local or network
    pub fn add_artifact_to_map(
        &mut self,
        package: SharedPackageConfig,
        overwrite_existing: bool,
    ) -> Result<()> {
        if !self.artifacts.contains_key(&package.config.info.id) {
            self.artifacts
                .insert(package.config.info.id.clone(), HashMap::new());
        }

        let id_artifacts = self.artifacts.get_mut(&package.config.info.id).unwrap();

        let entry = id_artifacts.entry(package.config.info.version.clone());

        match entry {
            Entry::Occupied(mut e) => {
                if overwrite_existing {
                    e.insert(package);
                }
            }
            Entry::Vacant(_) => {
                entry.insert_entry(package);
            }
        };

        Ok(())
    }

    /// for local qpm-rust installs
    pub fn add_artifact_and_cache(
        &mut self,
        package: SharedPackageConfig,
        project_folder: PathBuf,
        binary_path: Option<PathBuf>,
        debug_binary_path: Option<PathBuf>,
        copy: bool,
        overwrite_existing: bool,
    ) -> Result<()> {
        if copy {
            Self::copy_to_cache(
                &package,
                project_folder,
                binary_path,
                debug_binary_path,
                false,
            )?;
        }
        self.add_artifact_to_map(package, overwrite_existing)?;

        Ok(())
    }

    fn copy_to_cache(
        package: &SharedPackageConfig,
        project_folder: PathBuf,
        binary_path: Option<PathBuf>,
        debug_binary_path: Option<PathBuf>,
        validate: bool,
    ) -> Result<()> {
        println!(
            "Adding cache for local dependency {} {}",
            package.config.info.id.bright_red(),
            package.config.info.version.bright_green()
        );
        let config = get_combine_config();
        let cache_path = config
            .cache
            .as_ref()
            .unwrap()
            .join(&package.config.info.id)
            .join(package.config.info.version.to_string());

        let tmp_path = cache_path.join("tmp");
        let src_path = cache_path.join("src");

        if src_path.exists() {
            remove_dir_all(&src_path).context("Failed to remove existing src folder")?;
        }

        fs::create_dir_all(&src_path).context("Failed to create lib path")?;

        if binary_path.is_some() || debug_binary_path.is_some() {
            let lib_path = cache_path.join("lib");
            let so_path = lib_path.join(package.config.info.get_so_name());
            let debug_so_path =
                lib_path.join(format!("debug_{}", package.config.info.get_so_name()));

            if let Some(binary_path_unwrapped) = &binary_path {
                copy_things(binary_path_unwrapped, &so_path)?;
            }

            if let Some(debug_binary_path_unwrapped) = &debug_binary_path {
                copy_things(debug_binary_path_unwrapped, &debug_so_path)?;
            }
        }

        let original_shared_path = project_folder.join(&package.config.shared_dir);
        let original_package_file_path = project_folder.join("qpm.json");

        copy_things(
            &original_shared_path,
            &src_path.join(&package.config.shared_dir),
        )?;
        copy_things(&original_package_file_path, &src_path.join("qpm.json"))?;

        // if the tmp path exists, but src doesn't, that's a failed cache, delete it and try again!
        if tmp_path.exists() {
            remove_dir_all(&tmp_path).context("Failed to remove existing tmp folder")?;
        }

        if validate {
            let package_path = src_path;
            let downloaded_package = SharedPackageConfig::read(&package_path)?;

            // check if downloaded config is the same version as expected, if not, panic
            if downloaded_package.config.info.version != package.config.info.version {
                bail!(
                    "Downloaded package ({}) version ({}) does not match expected version ({})!",
                    package.config.info.id.bright_red(),
                    downloaded_package
                        .config
                        .info
                        .version
                        .to_string()
                        .bright_green(),
                    package.config.info.version.to_string().bright_green(),
                )
            }
        }

        Ok(())
    }

    /// always gets the global config
    pub fn read() -> Result<Self> {
        let path = Self::global_file_repository_path();
        fs::create_dir_all(Self::global_repository_dir())
            .context("Failed to make config folder")?;

        if let Ok(file) = std::fs::File::open(path) {
            Ok(serde_json::from_reader(&file)?)
        } else {
            // didn't exist
            Ok(Self::default())
        }
    }

    pub fn write(&self) -> Result<()> {
        let config = serde_json::to_string_pretty(&self).expect("Serialization failed");
        let path = Self::global_file_repository_path();

        std::fs::create_dir_all(Self::global_repository_dir())
            .context("Failed to make config folder")?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(config.as_bytes())?;
        println!("Saved local repository Config!");
        Ok(())
    }

    pub fn global_file_repository_path() -> PathBuf {
        Self::global_repository_dir().join("qpm.repository.json")
    }

    pub fn global_repository_dir() -> PathBuf {
        dirs::config_dir().unwrap().join("QPM-Rust")
    }

    fn collect_deps(
        &self,
        package: &PackageConfig,
        restored_deps: &[SharedPackageConfig],
    ) -> Result<HashMap<PathBuf, PathBuf>> {
        // let package = shared_package.config;
        let restored_dependencies_map: HashMap<&String, &SharedPackageConfig> = restored_deps
            .iter()
            .map(|p| (&p.config.info.id, p))
            .collect();

        let user_config = get_combine_config();
        let base_path = user_config.cache.as_ref().unwrap();

        let project_deps_path = Path::new(".")
            .join(&package.dependencies_dir)
            .canonicalize()?;
        let project_deps_binaries = project_deps_path.join("libs");
        let project_deps_headers = project_deps_path.join("includes");

        let mut paths = HashMap::<PathBuf, PathBuf>::new();
        // direct deps (binaries)
        for referenced_dep in package.dependencies.iter() {
            let shared_dep = restored_dependencies_map.get(&referenced_dep.id).unwrap();
            let dep_cache_path = base_path
                .join(&referenced_dep.id)
                .join(shared_dep.config.info.version.to_string());
            let _src_path = dep_cache_path.join("src");
            let libs_path = dep_cache_path.join("lib");

            // skip header only deps
            if shared_dep
                .config
                .additional_data
                .headers_only
                .unwrap_or(false)
            {
                continue;
            }

            if shared_dep.config.additional_data.so_link.is_some()
                || shared_dep.config.additional_data.debug_so_link.is_some()
            {
                // get so name or release so name
                let name = match shared_dep
                    .config
                    .additional_data
                    .use_release
                    .unwrap_or(false)
                {
                    true => shared_dep.config.info.get_so_name(),
                    false => format!("debug_{}", shared_dep.config.info.get_so_name()),
                };
                paths.insert(
                    libs_path.with_file_name(name.clone()),
                    project_deps_binaries.with_file_name(name),
                );
            }
        }

        // Get headers of all dependencies restored
        for (restored_id, restored_dep) in restored_dependencies_map {
            let dep_cache_path = base_path
                .join(restored_id)
                .join(restored_dep.config.info.version.to_string());
            let src_path = dep_cache_path.join("src");

            let exposed_headers = src_path.join(&restored_dep.config.shared_dir);
            let project_deps_headers_target = project_deps_headers.join(restored_id);

            paths.insert(
                exposed_headers,
                project_deps_headers_target.join(&restored_dep.config.shared_dir),
            );

            if let Some(extras) = &restored_dep.config.additional_data.extra_files {
                for extra in extras {
                    paths.insert(
                        project_deps_headers.join(extra),
                        project_deps_headers_target.join(extra),
                    );
                }
            }
        }

        paths.retain(|src, _| src.exists());

        Ok(paths)
    }

    pub fn pull_from_cache(&self, _shared_package: &SharedPackageConfig) {}
}

impl Repository for FileRepository {
    fn get_package_versions(&self, id: &str) -> Result<Option<Vec<PackageVersion>>> {
        Ok(self.get_artifacts_from_id(id).map(|artifacts| {
            artifacts
                .keys()
                .map(|version| PackageVersion {
                    id: id.to_string(),
                    version: version.clone(),
                })
                .collect()
        }))
    }

    fn get_package(&self, id: &str, version: &Version) -> Result<Option<SharedPackageConfig>> {
        Ok(self.get_artifact(id, version).cloned())
    }

    fn get_package_names(&self) -> Result<Vec<String>> {
        Ok(self.artifacts.keys().cloned().collect())
    }

    fn add_to_db_cache(&mut self, config: SharedPackageConfig, permanent: bool) -> Result<()> {
        if !permanent {
            return Ok(());
        }

        // don't copy files to cache
        // don't overwrite cache with backend
        self.add_artifact_to_map(config, false)?;
        Ok(())
    }

    fn download_to_cache(&mut self, config: &SharedPackageConfig) -> Result<()> {
        if self
            .get_artifact(&config.config.info.id, &config.config.info.version)
            .is_none()
        {
            bail!(
                "Local cache does not have {}:{}",
                config.config.info.id,
                config.config.info.version
            );
        }

        Ok(())
    }
}
