use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::Entry, HashMap},
    fs,
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

use qpm_package::models::{
    backend::PackageVersion,
    dependency::{SharedDependency, SharedPackageConfig},
    extra::DependencyLibType,
    package::PackageConfig,
};

use crate::{
    models::{config::get_combine_config, package::PackageConfigExtensions},
    terminal::colors::QPMColor,
    utils::{fs::copy_things, json},
};

use super::Repository;
use crate::models::package_dependeny::DependencyExtensions;

// TODO: Somehow make a global singleton of sorts/cached instance to share across places
// like resolver
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FileRepository {
    #[serde(default)]
    pub artifacts: HashMap<String, HashMap<Version, SharedPackageConfig>>,
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

    /// for local qpm-rs installs
    pub fn add_artifact_and_cache(
        &mut self,
        package: SharedPackageConfig,
        project_folder: &Path,
        binary_path: Option<&Path>,
        static_binary_path: Option<&Path>,
        copy: bool,
        overwrite_existing: bool,
    ) -> Result<()> {
        if copy {
            Self::copy_to_cache(
                &package,
                project_folder,
                binary_path,
                static_binary_path,
                false,
            )?;
        }
        self.add_artifact_to_map(package, overwrite_existing)?;

        Ok(())
    }

    fn copy_to_cache(
        shared_package: &SharedPackageConfig,
        project_folder: &Path,
        binary_path: Option<&Path>,
        static_binary_path: Option<&Path>,
        validate: bool,
    ) -> Result<()> {
        println!(
            "Adding cache for local dependency {} {}",
            shared_package.config.info.id.bright_red(),
            shared_package.config.info.version.bright_green()
        );
        let config = get_combine_config();
        let cache_path = config
            .cache
            .as_ref()
            .unwrap()
            .join(&shared_package.config.info.id)
            .join(shared_package.config.info.version.to_string());

        let tmp_path = cache_path.join("tmp");
        let src_path = cache_path.join("src");

        if src_path.exists() {
            fs::remove_dir_all(&src_path).context("Failed to remove existing src folder")?;
        }

        fs::create_dir_all(&src_path).context("Failed to create lib path")?;

        let lib_path = cache_path.join("lib");

        if let Some(binary_path_unwrapped) = &binary_path {
            let dynamic_out_name = shared_package.get_dynamic_lib_out()?.file_name().unwrap();
            let so_path = lib_path.join(dynamic_out_name);

            copy_things(binary_path_unwrapped, &so_path)?;
        }

        if let Some(static_binary_path_unwrapped) = &static_binary_path {
            let static_out_name = shared_package.get_static_lib_out()?.file_name().unwrap();
            let static_out_path = lib_path.join(static_out_name);
            copy_things(static_binary_path_unwrapped, &static_out_path)?;
        }

        let original_shared_path = project_folder.join(&shared_package.config.shared_dir);

        copy_things(
            &original_shared_path,
            &src_path.join(&shared_package.config.shared_dir),
        )?;
        copy_things(&project_folder.join("qpm.json"), &src_path.join("qpm.json"))?;
        copy_things(
            &project_folder.join("qpm.shared.json"),
            &src_path.join("qpm.shared.json"),
        )?;

        // if the tmp path exists, but src doesn't, that's a failed cache, delete it and try again!
        if tmp_path.exists() {
            fs::remove_dir_all(&tmp_path).context("Failed to remove existing tmp folder")?;
        }

        if validate {
            let package_path = src_path;
            let downloaded_package = SharedPackageConfig::read(package_path)?;

            // check if downloaded config is the same version as expected, if not, panic
            if downloaded_package.config.info.version != shared_package.config.info.version {
                bail!(
                    "Downloaded package ({}) version ({}) does not match expected version ({})!",
                    shared_package.config.info.id.bright_red(),
                    downloaded_package
                        .config
                        .info
                        .version
                        .to_string()
                        .bright_green(),
                    shared_package
                        .config
                        .info
                        .version
                        .to_string()
                        .bright_green(),
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
            json::json_from_reader_fast(BufReader::new(file))
                .context("Unable to read local repository config")
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
        dirs::config_dir().unwrap().join("QPM-RS")
    }

    pub fn clear() -> Result<(), std::io::Error> {
        fs::remove_file(Self::global_file_repository_path())
    }

    pub fn copy_from_cache(
        shared_package: &SharedPackageConfig,
        restored_deps: &[SharedPackageConfig],
        workspace_dir: &Path,
    ) -> Result<()> {
        let files = Self::collect_deps(shared_package, restored_deps, workspace_dir)?;

        let config = get_combine_config();
        let symlink = config.symlink.unwrap_or(true);

        let copy_dir_options = fs_extra::dir::CopyOptions {
            overwrite: true,
            copy_inside: true,
            content_only: true,
            ..Default::default()
        };

        let copy_file_options = fs_extra::file::CopyOptions {
            overwrite: true,
            ..Default::default()
        };

        for (src, dest) in files {
            fs::create_dir_all(dest.parent().unwrap())?;
            let symlink_result = if symlink {
                if src.is_file() {
                    symlink::symlink_file(&src, &dest)
                } else {
                    symlink::symlink_dir(&src, &dest)
                }
            } else {
                Ok(())
            };

            if let Err(e) = &symlink_result {
                #[cfg(windows)]
                eprintln!("Failed to create symlink: {}\nfalling back to copy, did the link already exist, or did you not enable windows dev mode?\nTo disable this warning (and default to copy), use the command {}", e.bright_red(), "qpm config symlink disable".bright_yellow());
                #[cfg(not(windows))]
                eprintln!("Failed to create symlink: {}\nfalling back to copy, did the link already exist?\nTo disable this warning (and default to copy), use the command {}", e.bright_red(), "qpm config symlink disable".bright_yellow());
            }

            if !symlink || symlink_result.is_err() {
                // if dir, make sure it exists
                if !src.exists() {
                    bail!("The file or folder\n\t'{}'\ndid not exist! what happened to the cache? you should probably run {} to make sure everything is in order...", src.display().bright_yellow(), "qpm cache clear".bright_yellow());
                } else if src.is_dir() {
                    std::fs::create_dir_all(&dest)
                        .context("Failed to create destination folder")?;

                    // copy it over
                    fs_extra::dir::copy(&src, &dest, &copy_dir_options)?;
                } else if src.is_file() {
                    // if it's a file, copy that over instead

                    fs_extra::file::copy(&src, &dest, &copy_file_options)?;
                }
            }
        }
        Ok(())
    }

    pub fn collect_deps(
        shared_package: &SharedPackageConfig,
        restored_deps: &[SharedPackageConfig],
        workspace_dir: &Path,
    ) -> Result<HashMap<PathBuf, PathBuf>> {
        let package = &shared_package.config;
        let restored_dependencies_map: HashMap<&String, &SharedPackageConfig> = restored_deps
            .iter()
            .map(|p| (&p.config.info.id, p))
            .collect();
        let locked_dependencies_map: HashMap<&String, &SharedDependency> = shared_package
            .restored_dependencies
            .iter()
            .map(|p| (&p.dependency.id, p))
            .collect();

        let user_config = get_combine_config();
        let base_path = user_config.cache.as_ref().unwrap();

        // validate exists dependencies
        let missing_dependencies: Vec<_> = restored_dependencies_map
            .iter()
            .filter(|(_, r)| {
                !base_path
                    .join(&r.config.info.id)
                    .join(r.config.info.version.to_string())
                    .exists()
            })
            .map(|(_, r)| format!("{}:{}", r.config.info.id, r.config.info.version))
            .collect();

        if !missing_dependencies.is_empty() {
            bail!("Missing dependencies in cache: {:?}", missing_dependencies);
        }

        let extern_dir = workspace_dir.join(&package.dependencies_dir);

        let extern_binaries = extern_dir.join("libs");
        let extern_headers = extern_dir.join("includes");

        let headers = locked_dependencies_map.iter().map(
            |(dep_id, shared_dep)| -> Result<(PathBuf, PathBuf)> {
                let shared_dep_config =
                    restored_dependencies_map.get(dep_id).unwrap_or_else(|| {
                        panic!(
                            "No shared config in resolved_deps for dependency {}:{}",
                            dep_id.dependency_id_color(),
                            shared_dep.version.dependency_version_color()
                        )
                    });

                let dep_cache_path = base_path
                    .join(dep_id)
                    .join(shared_dep_config.config.info.version.to_string());

                let src_path = dep_cache_path.join("src");

                if !src_path.exists() {
                    bail!(
                        "Missing src for dependency {}:{}",
                        dep_id,
                        shared_dep_config.config.info.version.to_string()
                    );
                }

                let exposed_headers = src_path.join(&shared_dep_config.config.shared_dir);
                let project_deps_headers_target = extern_headers.join(dep_id);

                let path = (
                    exposed_headers,
                    project_deps_headers_target.join(&shared_dep_config.config.shared_dir),
                );

                Ok(path)
            },
        );

        let binaries = locked_dependencies_map
            .iter()
            .filter(|(_dep_id, shared_dep)| shared_dep.restored_lib_type != DependencyLibType::HeaderOnly)
            .map(|(dep_id, shared_dep)| -> Result<(PathBuf, PathBuf)> {
                let data = &shared_dep.dependency.additional_data;

                let dep_cache_path = base_path
                    .join(dep_id)
                    .join(shared_dep.version.to_string());
                let libs_path = dep_cache_path.join("lib");

                let src_path = dep_cache_path.join("src");

                if !src_path.exists() {
                    bail!(
                        "Missing src for dependency {}:{}",
                        dep_id,
                        shared_dep.version.to_string()
                    );
                }

                let dependency_lib_type = shared_dep.restored_lib_type.clone();
                let name = match dependency_lib_type {
                    // if has so link and is not using static_link
                    // use so name
                    DependencyLibType::Shared => {
                        shared_dep.dependency.get_dynamic_lib_out()?.file_name().unwrap().to_str().unwrap()
                    }
                    DependencyLibType::Static => {
                        shared_dep.dependency.get_static_lib_out()?.file_name().unwrap().to_str().unwrap()
                    }
                    _ => bail!("Attempting to use dependency as {dependency_lib_type:?} but failed. Info: {data:?}"),
                };

                let src_binary = libs_path.join(name);
                let dst_binary =
                    extern_binaries.join(name);

                if !src_binary.exists() {
                    bail!(
                        "Missing binary {} for {}:{}",
                        name,
                        dep_id,
                        shared_dep.version
                    );
                }

                let path = (src_binary, dst_binary);
                Ok(path)
            });

        // extra files
        let extra_files = package
            .dependencies
            .iter()
            .map(|referenced_dependency| {
                let shared_dep = restored_dependencies_map
                    .get(&referenced_dependency.id)
                    .unwrap();

                let dep_cache_path = base_path
                    .join(&referenced_dependency.id)
                    .join(shared_dep.config.info.version.to_string());
                let src_path = dep_cache_path.join("src");

                let mut paths: Vec<(PathBuf, PathBuf)> = vec![];

                let extern_headers_dep = extern_headers.join(&referenced_dependency.id);
                if let Some(extras) = &referenced_dependency.additional_data.extra_files {
                    for extra in extras {
                        let extra_src = src_path.join(extra);

                        if !extra_src.exists() {
                            bail!(
                                "Missing extra {extra} for dependency {}:{}",
                                referenced_dependency.id,
                                shared_dep.config.info.version.to_string()
                            );
                        }

                        paths.push((extra_src, extern_headers_dep.join(extra)));
                    }
                }

                Ok(paths)
            })
            .flatten_ok();

        let mut paths: HashMap<PathBuf, PathBuf> =
            headers.chain(binaries).chain(extra_files).try_collect()?;

        paths.retain(|src, _| src.exists());

        Ok(paths)
    }
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
                .sorted_by(|a, b| a.version.cmp(&b.version))
                .rev() // highest first
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

    fn download_to_cache(&mut self, config: &PackageConfig) -> Result<bool> {
        let exist_in_db = self
            .get_artifact(&config.info.id, &config.info.version)
            .is_some();
        let exists_in_cache = get_combine_config()
            .cache
            .as_ref()
            .unwrap()
            .join(&config.info.id)
            .join(config.info.version.to_string())
            .exists();
        Ok(exist_in_db && exists_in_cache)
    }

    fn write_repo(&self) -> Result<()> {
        self.write()
    }

    fn is_online(&self) -> bool {
        false
    }
}
