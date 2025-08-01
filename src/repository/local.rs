use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, OptionExt, bail, ensure},
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use qpm_package::models::{
    package::{DependencyId, PackageConfig, QPM_JSON},
    qpkg::{QPKG_JSON, QPkg},
    shared_package::QPM_SHARED_JSON,
    triplet::TripletId,
};
use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    fs,
    io::{BufReader, Read, Seek, Write},
    ops::Not,
    path::{Path, PathBuf},
};
use zip::ZipArchive;

use crate::{
    models::{
        config::get_combine_config,
        package::PackageConfigExtensions,
        package_files::PackageIdPath,
        qpkg::QPkgExtensions,
        schemas::{SchemaLinks, WithSchema},
    },
    resolver::dependency::ResolvedDependency,
    terminal::colors::QPMColor,
    utils::{fs::copy_things, json},
};

use super::Repository;

// All files must exist
pub struct PackageFiles {
    /// Paths to the header files of the package on the filesystem.
    pub headers: PathBuf,
    /// Paths to the binary files of the package on the filesystem.
    pub binaries: Vec<PathBuf>,
    // pub extras: Vec<PathBuf>,
}

// TODO: Somehow make a global singleton of sorts/cached instance to share across places
// like resolver
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Default)]
pub struct FileRepository {
    #[serde(default)]
    pub artifacts: HashMap<DependencyId, HashMap<Version, PackageConfig>>,
}

impl FileRepository {
    pub fn get_artifacts_from_id(
        &self,
        id: &DependencyId,
    ) -> Option<&HashMap<Version, PackageConfig>> {
        self.artifacts.get(id)
    }

    pub fn get_artifact(&self, id: &DependencyId, version: &Version) -> Option<&PackageConfig> {
        match self.artifacts.get(id) {
            Some(artifacts) => artifacts.get(version),
            None => None,
        }
    }

    /// for adding to cache from local or network
    pub fn add_artifact_to_map(
        &mut self,
        package: PackageConfig,
        overwrite_existing: bool,
    ) -> Result<()> {
        if !self.artifacts.contains_key(&package.id) {
            self.artifacts.insert(package.id.clone(), HashMap::new());
        }

        let id_artifacts = self.artifacts.get_mut(&package.id).unwrap();

        let entry = id_artifacts.entry(package.version.clone());

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
        package: PackageConfig,
        overwrite_existing: bool,
    ) -> Result<()> {
        self.add_artifact_to_map(package, overwrite_existing)?;

        Ok(())
    }

    #[deprecated(note = "Use qpkg_install instead")]
    pub fn copy_to_cache(
        package: &PackageConfig,
        triplet: &TripletId,
        project_folder: PathBuf,
        binaries: Vec<PathBuf>,
        validate: bool,
    ) -> Result<()> {
        println!(
            "Adding cache for local dependency {} {}",
            package.id.bright_red(),
            package.version.bright_green()
        );
        let cache_path = PackageIdPath(package.id.clone())
            .version(package.version.clone())
            .triplet(triplet.clone());

        let tmp_path = cache_path.tmp_path();
        if tmp_path.exists() {
            fs::remove_dir_all(&tmp_path).context("Failed to remove existing tmp folder")?;
        }

        if cache_path.src_path().exists() {
            fs::remove_dir_all(cache_path.src_path())
                .context("Failed to remove existing src folder")?;
        }

        fs::create_dir_all(cache_path.src_path()).context("Failed to create lib path")?;

        for binary_src in binaries {
            if !binary_src.exists() {
                bail!(
                    "Binary {} does not exist, cannot copy to cache!",
                    binary_src.display().bright_yellow()
                );
            }

            let binary_dst = cache_path
                .binaries_path()
                .join(binary_src.file_name().unwrap());

            copy_things(&binary_src, &binary_dst)?;
        }

        let original_shared_path = project_folder.join(&package.shared_directory);

        copy_things(
            &original_shared_path,
            &cache_path.src_path().join(&package.shared_directory),
        )?;
        copy_things(
            &project_folder.join(QPM_JSON),
            &cache_path.src_path().join(QPM_JSON),
        )?;
        copy_things(
            &project_folder.join(QPM_SHARED_JSON),
            &cache_path.src_path().join(QPM_SHARED_JSON),
        )?;

        // if the tmp path exists, but src doesn't, that's a failed cache, delete it and try again!
        if tmp_path.exists() {
            fs::remove_dir_all(&tmp_path).context("Failed to remove existing tmp folder")?;
        }

        if validate {
            let package_path = cache_path.src_path();
            let downloaded_package = PackageConfig::read(package_path)?;

            // check if downloaded config is the same version as expected, if not, panic
            if downloaded_package.version != package.version {
                bail!(
                    "Downloaded package ({}) version ({}) does not match expected version ({})!",
                    package.id.dependency_id_color(),
                    downloaded_package.version.red(),
                    package.version.green(),
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

    /// Returns the path to the global file repository
    pub fn global_file_repository_path() -> PathBuf {
        Self::global_repository_dir().join("qpm.repository.json")
    }

    /// Returns the global repository directory, which is usually in the user's config directory
    pub fn global_repository_dir() -> PathBuf {
        dirs::config_dir().unwrap().join("QPM-RS2")
    }

    pub fn clear() -> Result<(), std::io::Error> {
        fs::remove_file(Self::global_file_repository_path())
    }

    pub fn install_qpkg<T>(buffer: T, overwrite_existing: bool) -> color_eyre::Result<PackageConfig>
    where
        T: Read + Seek,
    {
        // Extract to tmp folder
        let mut zip_archive = ZipArchive::new(buffer).context("Reading zip")?;

        // get qpkg in memory
        let qpkg_file = zip_archive
            .by_name(QPKG_JSON)
            .with_context(|| format!("Failed to find {QPKG_JSON} in zip"))?;

        let qpkg: QPkg = json::json_from_reader_fast(qpkg_file)
            .with_context(|| format!("Failed to read {QPKG_JSON} from zip"))?;

        let package_path: crate::models::package_files::PackageVersionPath =
            PackageIdPath::new(qpkg.config.id.clone()).version(qpkg.config.version.clone());

        let tmp_path = package_path.tmp_path();
        let qpkg_file_dst = package_path.qpkg_json_path();
        let headers_dst = package_path.src_path();
        let base_path = package_path.base_path();

        if QPkg::exists(&base_path) {
            match overwrite_existing {
                false => {
                    bail!(
                        "QPKG already exists {}",
                        base_path.display().file_path_color()
                    );
                }
                true => {
                    println!(
                        "Overwriting existing QPKG {}",
                        base_path.display().file_path_color()
                    );
                }
            }
        }

        // copy QPKG.qpm.json to {cache}/{id}/{version}/src/qpm2.qpkg.json
        if base_path.exists() {
            fs::remove_dir_all(&base_path).with_context(|| {
                format!(
                    "Failed to remove existing QPkg at {}",
                    package_path.base_path().display().file_path_color()
                )
            })?;
        }
        fs::create_dir_all(&base_path).with_context(|| {
            format!(
                "Failed to create package path{}",
                package_path.base_path().display().file_path_color()
            )
        })?;

        // make tmp_path
        fs::create_dir_all(&tmp_path).with_context(|| {
            format!(
                "Failed to create tmp folder {}",
                tmp_path.display().file_path_color()
            )
        })?;

        // src did not exist, this means that we need to download the repo/zip file from packageconfig.url
        fs::create_dir_all(&headers_dst)
            .with_context(|| format!("Failed to create lib path {headers_dst:?}"))?;

        // now extract the zip to the tmp path
        zip_archive.extract(&tmp_path).context("Zip extraction")?;

        fs::rename(tmp_path.join(QPKG_JSON), &qpkg_file_dst).with_context(|| {
            format!(
                "Failed to copy QPkg file from {} to {}",
                tmp_path.display().file_path_color(),
                qpkg_file_dst.display().file_path_color()
            )
        })?;

        // copy headers to src folder
        fs::rename(tmp_path.join(&qpkg.shared_dir), &headers_dst).with_context(|| {
            format!(
                "Failed to copy headers from {} to {}",
                tmp_path.display().file_path_color(),
                headers_dst.display().file_path_color()
            )
        })?;

        // copy binaries to lib folder
        for (triplet_id, triplet_info) in &qpkg.triplets {
            let bin_dir = package_path
                .clone()
                .triplet(triplet_id.clone())
                .binaries_path();

            if !bin_dir.exists() {
                fs::create_dir_all(&bin_dir).context("Failed to create lib path")?;
            }

            for file in &triplet_info.files {
                let src_file = tmp_path.join(file);
                let dst_file = bin_dir.join(file.file_name().unwrap());
                // copy as {cache}/{id}/{version}/{triplet}/lib/{file_name}
                fs::rename(&src_file, &dst_file).with_context(|| {
                    format!(
                        "Failed to copy file from {} to {}",
                        src_file.display().file_path_color(),
                        dst_file.display().file_path_color()
                    )
                })?;
            }
        }

        // assert that the triplets binaries are present
        for (triplet_id, triplet) in qpkg.config.triplets.iter_triplets() {
            let triplet_path = package_path.clone().triplet(triplet_id.clone());
            let triplet_bin_path = triplet_path.binaries_path();
            if !triplet_bin_path.exists() {
                bail!(
                    "Triplet binaries for {} not found in {}",
                    triplet_id.triplet_id_color(),
                    triplet_bin_path.display().file_path_color()
                );
            }

            for binary in triplet.out_binaries.iter().flatten() {
                // {cache}/{id}/{version}/{triplet}/lib/{binary}
                let binary_path = triplet_path.binary_path(binary);
                if !binary_path.exists() {
                    bail!(
                        "Binary {} not found in triplet {} at {}",
                        binary.display().file_path_color(),
                        triplet_id.triplet_id_color(),
                        binary_path.display().file_path_color()
                    );
                }
            }
        }

        // now write the package config to the src path
        qpkg.config.write(&base_path).with_context(|| {
            format!(
                "Failed to write package config to {}",
                headers_dst.display().file_path_color()
            )
        })?;

        let mut file_repo = FileRepository::read()?;

        file_repo.add_artifact_and_cache(qpkg.config.clone(), true)?;
        file_repo.write()?;

        // write the qpkg file to the src path
        qpkg.write(&base_path).with_context(|| {
            format!(
                "Failed to write QPkg file to {}",
                headers_dst.display().file_path_color()
            )
        })?;

        // clear up tmp folder if it still exists
        if tmp_path.exists() {
            std::fs::remove_dir_all(tmp_path).context("Failed to remove tmp folder")?;
        }

        Ok(qpkg.config)
    }

    pub fn copy_from_cache(
        package: &PackageConfig,
        triplet: &TripletId,
        restored_deps: &[ResolvedDependency],
        workspace_dir: &Path,
    ) -> Result<()> {
        let files = Self::collect_deps(package, triplet, restored_deps, workspace_dir)?;

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

        let extern_dir = workspace_dir.join(&package.dependencies_directory);

        ensure!(extern_dir != workspace_dir, "Extern dir is workspace dir!");

        let extern_binaries = Self::libs_dir(&extern_dir);
        let extern_headers = Self::headers_path(&extern_dir);

        // delete if needed extern/libs and extern/includes
        if extern_binaries.exists() {
            fs::remove_dir_all(&extern_binaries)
                .with_context(|| format!("Unable to delete {extern_binaries:?}"))?;
        }
        if extern_headers.exists() {
            fs::remove_dir_all(&extern_headers)
                .with_context(|| format!("Unable to delete {extern_headers:?}"))?;
        }

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
                    "qpm2 config symlink disable".bright_yellow()
                );
                #[cfg(not(windows))]
                eprintln!(
                    "Failed to create symlink: {}\nfalling back to copy, did the link already exist?\nTo disable this warning (and default to copy), use the command {}",
                    e.bright_red(),
                    "qpm2 config symlink disable".bright_yellow()
                );
            }

            if !symlink || symlink_result.is_err() {
                // if dir, make sure it exists
                if !src.exists() {
                    bail!(
                        "The file or folder\n\t'{}'\ndid not exist! what happened to the cache? you should probably run {} to make sure everything is in order...",
                        src.display().bright_yellow(),
                        "qpm2 cache clear".bright_yellow()
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

    /// Collects all files of a package from the cache.
    /// Returns a `PackageFiles` struct containing the paths to the headers, release binary, and debug binary.
    pub fn collect_files_of_package(
        package: &PackageConfig,
        triplet: &TripletId,
    ) -> Result<PackageFiles> {
        let package_triplet = package
            .triplets
            .get_triplet_settings(triplet)
            .expect("Triplet settings not found");

        let dep_cache_path = PackageIdPath::new(package.id.clone())
            .version(package.version.clone())
            .triplet(triplet.clone());

        if !dep_cache_path.triplet_path().exists() {
            bail!(
                "Missing cache for dependency {}:{}",
                package.id.dependency_id_color(),
                package.version.dependency_version_color()
            );
        }

        let src_path = dep_cache_path.src_path();

        if !src_path.exists() {
            bail!(
                "Missing src for dependency {}:{}",
                package.id.dependency_id_color(),
                package.version.dependency_version_color()
            );
        }

        let exposed_headers = src_path.join(&package.shared_directory);

        let expected_binaries = package_triplet.out_binaries.unwrap_or_default();
        let binaries: Vec<PathBuf> = expected_binaries
            .iter()
            .map(|b| dep_cache_path.binary_path(b))
            .collect();

        // ensure no duplicates
        let mut seen = HashSet::new();
        for bin in &binaries {
            if !bin.exists() {
                bail!(
                    "Missing binary {} for dependency {}:{}",
                    bin.display().bright_yellow(),
                    package.id.dependency_id_color(),
                    package.version.dependency_version_color()
                );
            }

            if !seen.insert(bin.clone()) {
                bail!(
                    "Duplicate binary {} for dependency {}:{}",
                    bin.display().bright_yellow(),
                    package.id.dependency_id_color(),
                    package.version.dependency_version_color()
                );
            }
        }

        Ok(PackageFiles {
            headers: exposed_headers,
            binaries,
        })
    }

    pub fn libs_dir(extern_dir: &Path) -> PathBuf {
        extern_dir.join("libs")
    }

    pub fn headers_path(extern_dir: &Path) -> PathBuf {
        extern_dir.join("includes")
    }

    pub fn build_path(extern_dir: &Path) -> PathBuf {
        extern_dir.join("build")
    }

    /// Collects all dependencies of a package from the cache.
    /// Returns a map of source paths to target paths for the dependencies.
    pub fn collect_deps(
        package: &PackageConfig,
        triplet: &TripletId,
        restored_deps: &[ResolvedDependency],
        workspace_dir: &Path,
    ) -> Result<HashMap<PathBuf, PathBuf>> {
        let triplet_config = package
            .triplets
            .get_triplet_settings(triplet)
            .context("Failed to get triplet settings")?;

        // validate exists dependencies
        let missing_dependencies: Vec<_> = restored_deps
            .iter()
            .filter_map(|r| {
                let package_path = PackageIdPath::new(r.0.id.clone())
                    .version(r.0.version.clone())
                    .triplet(r.1.clone())
                    .triplet_path();
                // if the package path does not exist, return the id and version
                package_path
                    .exists()
                    .not()
                    .then_some(format!("{}:{}/{}", r.0.id, r.0.version, r.1))
            })
            .collect();

        if !missing_dependencies.is_empty() {
            bail!("Missing dependencies in cache: {:?}", missing_dependencies);
        }

        let extern_dir = workspace_dir.join(&package.dependencies_directory);

        ensure!(extern_dir != workspace_dir, "Extern dir is workspace dir!");

        let extern_binaries = Self::libs_dir(&extern_dir);
        let extern_headers = Self::headers_path(&extern_dir);

        let mut paths = HashMap::<PathBuf, PathBuf>::new();

        // direct deps (binaries)
        let deps: Vec<_> = restored_deps
            .iter()
            .map(|resolved_dep| -> color_eyre::Result<_> {
                let collect_files_of_package =
                    Self::collect_files_of_package(&resolved_dep.0, &resolved_dep.1)?;

                Ok((resolved_dep, collect_files_of_package))
            })
            .try_collect()?;

        let (direct_deps, indirect_deps): (Vec<_>, Vec<_>) =
            // partition by direct dependencies and indirect
            deps.into_iter().partition(|(unknown_dep, _)| {
                triplet_config
                    .dependencies
                    .iter()
                    .any(|direct_dep| *direct_dep.0 == unknown_dep.0.id)
            });

        // direct dependencies copy the binaries to the extern_binaries folder
        for (direct_dep, direct_dep_files) in &direct_deps {
            for binary in &direct_dep_files.binaries {
                let file_name = binary.file_name().expect("Failed to get file name");

                if !binary.exists() {
                    bail!(
                        "Missing binary {} for dependency {}:{}",
                        binary.display().bright_yellow(),
                        direct_dep.0.id.dependency_id_color(),
                        direct_dep.0.version.dependency_version_color()
                    );
                }

                // copy to extern/libs/{file_name}
                paths.insert(binary.clone(), extern_binaries.join(file_name));
            }
        }

        // Get headers of all dependencies restored
        for (dep, dep_files) in direct_deps.into_iter().chain(indirect_deps.into_iter()) {
            let project_deps_headers_target = extern_headers.join(dep.0.id.0.clone());

            let exposed_headers = dep_files.headers;
            if !exposed_headers.exists() {
                bail!(
                    "Missing header files for {}:{}",
                    dep.0.id.dependency_id_color(),
                    dep.0.version.dependency_version_color()
                );
            }

            paths.insert(exposed_headers, project_deps_headers_target);
        }

        paths.retain(|src, _| src.exists());

        // ensure no collisions
        let mut seen = HashSet::new();
        for (src, dest) in &paths {
            if !seen.insert(dest) {
                bail!(
                    "Collision detected for {} and {}",
                    src.display().bright_yellow(),
                    dest.display().bright_yellow()
                );
            }
        }

        Ok(paths)
    }

    pub fn remove_package_versions(&mut self, package: &DependencyId) -> Result<()> {
        self.artifacts.remove(package);
        let packages_path = PackageIdPath::new(package.clone()).versions_path();
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

        let packages_path = PackageIdPath::new(package.clone())
            .version(version.clone())
            .base_path();

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

    fn get_package(&self, id: &DependencyId, version: &Version) -> Result<Option<PackageConfig>> {
        Ok(self.get_artifact(id, version).cloned())
    }

    fn get_package_names(&self) -> Result<Vec<DependencyId>> {
        Ok(self.artifacts.keys().cloned().collect())
    }

    fn add_to_db_cache(&mut self, config: PackageConfig, permanent: bool) -> Result<()> {
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
        let package_path = PackageIdPath::new(config.id.clone()).version(config.version.clone());

        let config = PackageConfig::read(package_path.qpm_json_path());

        Ok(exist_in_db && package_path.src_path().exists() && config.is_ok())
    }

    fn write_repo(&self) -> Result<()> {
        self.write()
    }

    fn is_online(&self) -> bool {
        false
    }
}
