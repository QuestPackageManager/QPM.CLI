use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, OptionExt, bail, ensure},
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use qpm_package::models::{
    package::{DependencyId, PackageConfig},
    shared_package::SharedPackageConfig,
};
use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, hash_map::Entry},
    fs,
    io::{BufReader, Write},
    ops::Not,
    path::{Path, PathBuf},
};

use crate::{
    models::{
        config::get_combine_config,
        package::PackageConfigExtensions,
        schemas::{SchemaLinks, WithSchema},
    },
    terminal::colors::QPMColor,
    utils::{fs::copy_things, json},
};

use super::Repository;

// All files must exist
pub struct PackageFiles {
    pub headers: PathBuf,
    pub binary: Option<PathBuf>,
    // pub extras: Vec<PathBuf>,
}

// TODO: Somehow make a global singleton of sorts/cached instance to share across places
// like resolver
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Default)]
pub struct FileRepository {
    #[serde(default)]
    pub artifacts: HashMap<DependencyId, HashMap<Version, SharedPackageConfig>>,
}

impl FileRepository {
    pub fn get_artifacts_from_id(
        &self,
        id: &DependencyId,
    ) -> Option<&HashMap<Version, SharedPackageConfig>> {
        self.artifacts.get(id)
    }

    pub fn get_artifact(
        &self,
        id: &DependencyId,
        version: &Version,
    ) -> Option<&SharedPackageConfig> {
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
        if !self.artifacts.contains_key(&package.config.id) {
            self.artifacts
                .insert(package.config.id.clone(), HashMap::new());
        }

        let id_artifacts = self.artifacts.get_mut(&package.config.id).unwrap();

        let entry = id_artifacts.entry(package.config.version.clone());

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
            package.config.id.bright_red(),
            package.config.version.bright_green()
        );
        let config = get_combine_config();
        let cache_path = config
            .cache
            .as_ref()
            .unwrap()
            .join(&package.config.id.0)
            .join(package.config.version.to_string());

        let tmp_path = cache_path.join("tmp");
        let src_path = cache_path.join("src");

        if src_path.exists() {
            fs::remove_dir_all(&src_path).context("Failed to remove existing src folder")?;
        }

        fs::create_dir_all(&src_path).context("Failed to create lib path")?;

        if binary_path.is_some() || debug_binary_path.is_some() {
            let lib_path = cache_path.join("lib");
            let so_path = lib_path.join(package.config.get_so_name2());
            let debug_bin_name = package.config.get_so_name2().with_extension("debug.so");

            let debug_so_path = lib_path.join(debug_bin_name.file_name().unwrap());

            if let Some(binary_path_unwrapped) = &binary_path {
                copy_things(binary_path_unwrapped, &so_path)?;
            }

            if let Some(debug_binary_path_unwrapped) = &debug_binary_path {
                copy_things(debug_binary_path_unwrapped, &debug_so_path)?;
            }
        }

        let original_shared_path = project_folder.join(&package.config.shared_dir);

        copy_things(
            &original_shared_path,
            &src_path.join(&package.config.shared_directories),
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
            if downloaded_package.config.version != package.config.version {
                bail!(
                    "Downloaded package ({}) version ({}) does not match expected version ({})!",
                    package.config.id.bright_red(),
                    downloaded_package.config.version.to_string().bright_green(),
                    package.config.version.to_string().bright_green(),
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
        let config = serde_json::to_string_pretty(&WithSchema {
            schema: SchemaLinks::FILE_REPOSITORY,
            value: self,
        })
        .expect("Serialization failed");
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
        package: &PackageConfig,
        restored_deps: &[SharedPackageConfig],
        workspace_dir: &Path,
    ) -> Result<()> {
        let files = Self::collect_deps(package, restored_deps, workspace_dir)?;

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
                eprintln!(
                    "Failed to create symlink: {}\nfalling back to copy, did the link already exist, or did you not enable windows dev mode?\nTo disable this warning (and default to copy), use the command {}",
                    e.bright_red(),
                    "qpm config symlink disable".bright_yellow()
                );
                #[cfg(not(windows))]
                eprintln!(
                    "Failed to create symlink: {}\nfalling back to copy, did the link already exist?\nTo disable this warning (and default to copy), use the command {}",
                    e.bright_red(),
                    "qpm config symlink disable".bright_yellow()
                );
            }

            if !symlink || symlink_result.is_err() {
                // if dir, make sure it exists
                if !src.exists() {
                    bail!(
                        "The file or folder\n\t'{}'\ndid not exist! what happened to the cache? you should probably run {} to make sure everything is in order...",
                        src.display().bright_yellow(),
                        "qpm cache clear".bright_yellow()
                    );
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

    #[inline]
    pub fn get_package_versions_cache_path(id: &DependencyId) -> PathBuf {
        let user_config = get_combine_config();
        let base_path = user_config.cache.as_ref().unwrap();

        // cache/{id}
        base_path.join(id.0)
    }

    #[inline]
    pub fn get_package_cache_path(id: &DependencyId, version: &Version) -> PathBuf {
        // cache/{id}/{version}
        Self::get_package_versions_cache_path(id).join(version.to_string())
    }

    pub fn collect_files_of_package(package: &PackageConfig) -> Result<PackageFiles> {
        let dep_cache_path = Self::get_package_cache_path(&package.id, &package.version);

        if !dep_cache_path.exists() {
            bail!(
                "Missing cache for dependency {}:{}",
                package.id.dependency_id_color(),
                package.version.dependency_version_color()
            );
        }

        let libs_path = dep_cache_path.join("lib");
        let src_path = dep_cache_path.join("src");

        if !src_path.exists() {
            bail!(
                "Missing src for dependency {}:{}",
                package.id.dependency_id_color(),
                package.version.dependency_version_color()
            );
        }

        let exposed_headers = src_path.join(&package.shared_dir);

        if package.additional_data.headers_only.unwrap_or(false) {
            return Ok(PackageFiles {
                headers: exposed_headers,
                binary: None,
            });
        }

        // get so name or release so name

        let use_release_name = package.additional_data.debug_so_link.is_none()
            || package.additional_data.static_link.is_some();

        // get so name or release so name
        let name = match use_release_name {
            true => package
                .get_so_name2()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            false => {
                // lib{name}.debug.so
                let bin = package.get_so_name2().with_extension("debug.so");

                bin.file_name().unwrap().to_string_lossy().to_string()
            }
        };

        let binary = libs_path.join(&name);

        if !binary.exists() {
            bail!(
                "Missing binary {name} for {}:{}",
                package.id.dependency_id_color(),
                package.version.dependency_version_color()
            );
        }

        Ok(PackageFiles {
            headers: exposed_headers,
            binary: Some(binary),
        })
    }

    pub fn collect_deps(
        package: &PackageConfig,
        restored_deps: &[SharedPackageConfig],
        workspace_dir: &Path,
    ) -> Result<HashMap<PathBuf, PathBuf>> {
        // let package = shared_package.config;
        let restored_dependencies_map: HashMap<&DependencyId, &SharedPackageConfig> =
            restored_deps.iter().map(|p| (&p.config.id, p)).collect();

        // validate exists dependencies
        let missing_dependencies: Vec<_> = restored_dependencies_map
            .iter()
            .filter(|(_, r)| {
                !Self::get_package_cache_path(&r.config.id, &r.config.version).exists()
            })
            .map(|(_, r)| format!("{}:{}", r.config.id, r.config.version))
            .collect();

        if !missing_dependencies.is_empty() {
            bail!("Missing dependencies in cache: {:?}", missing_dependencies);
        }

        let extern_dir = workspace_dir.join(&package.dependencies_dir);

        ensure!(extern_dir != workspace_dir, "Extern dir is workspace dir!");

        // delete if needed
        if extern_dir.exists() {
            fs::remove_dir_all(&extern_dir)
                .with_context(|| format!("Unable to delete {extern_dir:?}"))?;
        }

        let extern_binaries = extern_dir.join("libs");
        let extern_headers = extern_dir.join("includes");
        let mut paths = HashMap::<PathBuf, PathBuf>::new();

        // direct deps (binaries)

        let deps: Vec<_> = restored_deps
            .iter()
            .map(|p| Self::collect_files_of_package(&p.config).map(|f| (p, f)))
            .try_collect()?;

        let (direct_deps, indirect_deps): (Vec<_>, Vec<_>) =
            // partition by direct dependencies and indirect
            deps.into_iter().partition(|(dep, _)| {
                package
                    .dependencies
                    .iter()
                    .any(|d| d.id == dep.config.id)
            });

        for (direct_dep, direct_dep_files) in direct_deps {
            let project_deps_headers_target = extern_headers.join(direct_dep.config.id.clone());

            let exposed_headers = direct_dep_files.headers;
            let not_header_only = direct_dep
                .config
                .additional_data
                .headers_only
                .unwrap_or(false)
                .not();
            let src_binary = not_header_only
                // not header only
                .then(|| {
                    direct_dep_files.binary.wrap_err_with(|| {
                        format!(
                            "Binary not found for direct package {}:{}",
                            direct_dep.config.id.dependency_id_color(),
                            direct_dep.config.version.dependency_version_color()
                        )
                    })
                })
                .transpose()?;

            if let Some(src_binary) = src_binary.as_ref()
                && !src_binary.exists()
            {
                bail!(
                    "Missing binary {} for {}:{}",
                    src_binary.file_name().unwrap_or_default().to_string_lossy(),
                    direct_dep.config.id.dependency_id_color(),
                    direct_dep.config.version.dependency_version_color()
                );
            }
            if !exposed_headers.exists() {
                bail!(
                    "Missing header files for {}:{}",
                    direct_dep.config.id.dependency_id_color(),
                    direct_dep.config.version.dependency_version_color()
                );
            }

            if let Some(src_binary) = src_binary {
                let file_name = src_binary.file_name().expect("Failed to get file name");

                paths.insert(src_binary.clone(), extern_binaries.join(file_name));
            }

            paths.insert(
                exposed_headers,
                project_deps_headers_target.join(&direct_dep.config.shared_dir),
            );
        }

        // Get headers of all dependencies restored
        for (indirect_dep, indirect_dep_files) in indirect_deps {
            let project_deps_headers_target = extern_headers.join(indirect_dep.config.id.clone());

            let exposed_headers = indirect_dep_files.headers;
            if !exposed_headers.exists() {
                bail!(
                    "Missing header files for {}:{}",
                    indirect_dep.config.id.dependency_id_color(),
                    indirect_dep.config.version.dependency_version_color()
                );
            }

            paths.insert(
                exposed_headers,
                project_deps_headers_target.join(&indirect_dep.config.shared_dir),
            );
        }

        // extra files
        // while this is looped twice, generally I'd assume the compiler to properly
        // optimize this and it's better readability
        for referenced_dependency in &package.dependencies {
            let shared_dep = restored_dependencies_map
                .get(&referenced_dependency.id)
                .unwrap();

            let dep_cache_path =
                Self::get_package_cache_path(&referenced_dependency.id, &shared_dep.config.version);
            let src_path = dep_cache_path.join("src");

            let extern_headers_dep = extern_headers.join(&referenced_dependency.id);

            if let Some(extras) = &referenced_dependency.additional_data.extra_files {
                for extra in extras {
                    let extra_src = src_path.join(extra);

                    if !extra_src.exists() {
                        bail!(
                            "Missing extra {extra} for dependency {}:{}",
                            referenced_dependency.id,
                            shared_dep.config.version.to_string()
                        );
                    }

                    paths.insert(extra_src, extern_headers_dep.join(extra));
                }
            }
        }

        paths.retain(|src, _| src.exists());

        Ok(paths)
    }

    pub fn remove_package_versions(&mut self, package: &DependencyId) -> Result<()> {
        self.artifacts.remove(package);
        let packages_path = Self::get_package_versions_cache_path(package);
        if !packages_path.exists() {
            return Ok(());
        }
        std::fs::remove_dir_all(packages_path)?;
        Ok(())
    }
    pub fn remove_package(&mut self, package: &DependencyId, version: &Version) -> Result<()> {
        self.artifacts
            .get_mut(package)
            .ok_or_eyre(format!("No package found {package}/{version}"))?
            .remove(version);

        let packages_path = Self::get_package_cache_path(package, version);
        if !packages_path.exists() {
            return Ok(());
        }
        std::fs::remove_dir_all(packages_path)?;
        Ok(())
    }
}

impl Repository for FileRepository {
    fn get_package_versions(&self, id: &DependencyId) -> Result<Option<Vec<Version>>> {
        Ok(self.get_artifacts_from_id(id).map(|artifacts| {
            artifacts
                .keys()
                .sorted()
                .rev() // highest first
                .cloned()
                .collect()
        }))
    }

    fn get_package(
        &self,
        id: &DependencyId,
        version: &Version,
    ) -> Result<Option<SharedPackageConfig>> {
        Ok(self.get_artifact(id, version).cloned())
    }

    fn get_package_names(&self) -> Result<Vec<DependencyId>> {
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
        let exist_in_db = self.get_artifact(&config.id, &config.version).is_some();
        let file = FileRepository::collect_files_of_package(config);

        Ok(exist_in_db && file.is_ok())
    }

    fn write_repo(&self) -> Result<()> {
        self.write()
    }

    fn is_online(&self) -> bool {
        false
    }
}
