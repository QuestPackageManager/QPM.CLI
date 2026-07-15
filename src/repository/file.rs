use color_eyre::{
    Result,
    eyre::{Context, OptionExt, bail, ensure},
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use qpm_package::models::{
    package::{DependencyId, PackageConfig, QPM_JSON},
    qpkg::QPkg,
    shared_package::QPM_SHARED_JSON,
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

use sha2::{Digest, Sha256};

use crate::{
    models::{
        config::get_combine_config,
        package::PackageConfigExtensions,
        package_files::PackageIdPath,
        qpkg::QPkgExtensions,
        qpkg_file::QpkgFile,
        schemas::{SchemaLinks, WithSchema},
    },
    services::{network::download_bytes, pubgrub::ResolvedDependency},
    terminal::colors::QPMColor,
    utils::{fs::copy_things, json},
};

use super::{Artifact, Repository};

// All files must exist
pub struct PackageFiles {
    /// Paths to the header files of the package on the filesystem.
    pub headers: PathBuf,
    /// Paths to the binary files of the package on the filesystem.
    pub binaries: Vec<PathBuf>,
    // pub extras: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Default)]
pub struct FileRepositoryRegistry {
    #[serde(default)]
    pub artifacts: HashMap<DependencyId, HashMap<Version, Artifact>>,
}

// TODO: Somehow make a global singleton of sorts/cached instance to share across places
// like resolver
#[derive(Clone, Debug, Default)]
pub struct FileRepository {
    root: PathBuf,
    registry: FileRepositoryRegistry,
}

impl FileRepository {
    /// Builds a repository instance directly from a root and registry, bypassing disk I/O.
    /// Mainly useful for tests.
    pub fn new(root: PathBuf, registry: FileRepositoryRegistry) -> Self {
        Self { root, registry }
    }

    pub fn artifacts(&self) -> &HashMap<DependencyId, HashMap<Version, Artifact>> {
        &self.registry.artifacts
    }

    pub fn artifacts_mut(&mut self) -> &mut HashMap<DependencyId, HashMap<Version, Artifact>> {
        &mut self.registry.artifacts
    }

    /// The root cache directory this instance installs packages under and resolves paths from
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn get_artifacts_from_id(&self, id: &DependencyId) -> Option<&HashMap<Version, Artifact>> {
        self.registry.artifacts.get(id)
    }

    pub fn get_artifact(&self, id: &DependencyId, version: &Version) -> Option<&Artifact> {
        self.registry.artifacts.get(id)?.get(version)
    }

    /// for adding to cache from local or network
    pub fn add_artifact_to_map(
        &mut self,
        package: PackageConfig,
        qpkg_checksum: Option<String>,
        overwrite_existing: bool,
    ) -> Result<()> {
        let id_artifacts = self
            .registry
            .artifacts
            .entry(package.id.clone())
            .or_default();

        let entry = id_artifacts.entry(package.version.clone());
        let artifact = Artifact {
            config: package,
            qpkg_checksum,
        };

        match entry {
            Entry::Occupied(mut e) => {
                if overwrite_existing {
                    e.insert(artifact);
                }
            }
            Entry::Vacant(e) => {
                e.insert(artifact);
            }
        };

        Ok(())
    }

    /// for local qpm-rs installs
    pub fn add_artifact_and_cache(
        &mut self,
        package: PackageConfig,
        qpkg_checksum: Option<String>,
        overwrite_existing: bool,
    ) -> Result<()> {
        self.add_artifact_to_map(package, qpkg_checksum, overwrite_existing)?;

        Ok(())
    }

    #[deprecated(note = "Use qpkg_install instead")]
    pub fn copy_to_cache(
        &self,
        package: &PackageConfig,
        project_folder: PathBuf,
        binaries: Vec<PathBuf>,
        validate: bool,
    ) -> Result<()> {
        println!(
            "Adding cache for local dependency {} {}",
            package.id.bright_red(),
            package.version.bright_green()
        );
        let cache_root = &self.root;
        let cache_path = PackageIdPath::new(package.id.clone()).version(package.version.clone());

        let tmp_path = cache_path.tmp_path(&cache_root);
        if tmp_path.exists() {
            fs::remove_dir_all(&tmp_path).context("Failed to remove existing tmp folder")?;
        }

        if cache_path.src_path(&cache_root).exists() {
            fs::remove_dir_all(cache_path.src_path(&cache_root))
                .context("Failed to remove existing src folder")?;
        }

        fs::create_dir_all(cache_path.src_path(&cache_root))
            .context("Failed to create lib path")?;

        for binary_src in binaries {
            if !binary_src.exists() {
                bail!(
                    "Binary {} does not exist, cannot copy to cache!",
                    binary_src.display().bright_yellow()
                );
            }

            let binary_dst = cache_path
                .binaries_path(&cache_root)
                .join(binary_src.file_name().unwrap());

            copy_things(&binary_src, &binary_dst)?;
        }

        let original_shared_path = project_folder.join(&package.shared_directory);

        copy_things(
            &original_shared_path,
            &cache_path
                .src_path(&cache_root)
                .join(&package.shared_directory),
        )?;
        copy_things(
            &project_folder.join(QPM_JSON),
            &cache_path.src_path(&cache_root).join(QPM_JSON),
        )?;
        copy_things(
            &project_folder.join(QPM_SHARED_JSON),
            &cache_path.src_path(&cache_root).join(QPM_SHARED_JSON),
        )?;

        // if the tmp path exists, but src doesn't, that's a failed cache, delete it and try again!
        if tmp_path.exists() {
            fs::remove_dir_all(&tmp_path).context("Failed to remove existing tmp folder")?;
        }

        if validate {
            let package_path = cache_path.src_path(&cache_root);
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

    /// Reads (or defaults) the file repository registry, rooted at `root` for package path
    /// resolution
    pub fn read(root: PathBuf) -> Result<Self> {
        let path = Self::global_file_repository_path();
        fs::create_dir_all(Self::global_repository_dir())
            .context("Failed to make config folder")?;

        let registry = if let Ok(file) = std::fs::File::open(path) {
            json::json_from_reader_fast(BufReader::new(file))
                .context("Unable to read local repository config")?
        } else {
            // didn't exist
            FileRepositoryRegistry::default()
        };

        Ok(Self { root, registry })
    }

    /// Reads the file repository rooted at the user's configured global cache directory
    pub fn read_global_cache() -> Result<Self> {
        Self::read(Self::cache_root())
    }

    pub fn write(&self) -> Result<()> {
        let config = serde_json::to_string_pretty(&WithSchema {
            schema: SchemaLinks::FILE_REPOSITORY,
            value: &self.registry,
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

    /// Returns the root cache directory packages are installed under
    fn cache_root() -> PathBuf {
        get_combine_config()
            .cache
            .clone()
            .expect("No cache path set")
    }

    pub fn install_qpkg<T>(
        &mut self,
        mut buffer: T,
        overwrite_existing: bool,
        version: Option<Version>,
    ) -> color_eyre::Result<Artifact>
    where
        T: Read + Seek,
    {
        // Hash the raw archive before extracting it, so the checksum reflects exactly what was
        // installed. Streamed directly into the hasher instead of buffering into a Vec first -
        // zip's central directory lives at the end of the file, so the archive still needs a
        // separate (seekable) pass for parsing, but this pass itself needs no extra allocation.
        buffer
            .seek(std::io::SeekFrom::Start(0))
            .context("Failed to seek in buffer")?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut buffer, &mut hasher).context("Failed to hash QPKG contents")?;
        let qpkg_checksum = hex::encode(hasher.finalize());

        buffer
            .seek(std::io::SeekFrom::Start(0))
            .context("Failed to seek in buffer")?;

        // Open and read QPKG
        let qpkg_file = QpkgFile::open(buffer).context("Failed to read QPKG")?;

        // Apply version override if provided
        let mut qpkg = qpkg_file.manifest().clone();
        if let Some(version) = version {
            qpkg.config.version = version;
        }

        let package_path: crate::models::package_files::PackageVersionPath =
            PackageIdPath::new(qpkg.config.id.clone()).version(qpkg.config.version.clone());

        let cache_root = &self.root;
        let base_path = package_path.base_path(&cache_root);
        let qpkg_file_dst = package_path.qpkg_json_dir(&cache_root);
        let headers_dst = package_path.src_path(&cache_root);
        let bin_dir = package_path.binaries_path(&cache_root);

        // Check if already exists
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
                    fs::remove_dir_all(&base_path).with_context(|| {
                        format!(
                            "Failed to remove existing QPkg at {}",
                            base_path.display().file_path_color()
                        )
                    })?;
                }
            }
        }

        // Create cache directories
        fs::create_dir_all(&base_path).with_context(|| {
            format!(
                "Failed to create package path {}",
                base_path.display().file_path_color()
            )
        })?;

        println!(
            "Installing package with {} files",
            qpkg.files.len().file_path_color()
        );

        // Extract QPKG to cache
        let extracted_config = qpkg_file
            .extract_to(&qpkg_file_dst, &headers_dst, &bin_dir)
            .context("Failed to extract QPKG")?;

        // Validate binaries exist
        for binary in qpkg.config.workspace.out_binaries.iter().flatten() {
            let binary_path = package_path.binary_path(&cache_root, binary);
            if !binary_path.exists() {
                bail!(
                    "Binary {} not found at {}",
                    binary.display().file_path_color(),
                    binary_path.display().file_path_color()
                );
            }
        }

        // Write package config to cache
        let config_path = headers_dst.join(QPM_JSON);
        let config_json = serde_json::to_string_pretty(&extracted_config)
            .context("Failed to serialize package config")?;
        fs::write(&config_path, config_json).with_context(|| {
            format!(
                "Failed to write package config to {}",
                config_path.display().file_path_color()
            )
        })?;

        // Update repository index
        self.add_artifact_and_cache(extracted_config.clone(), Some(qpkg_checksum.clone()), true)?;
        self.write()?;

        Ok(Artifact {
            config: extracted_config,
            qpkg_checksum: Some(qpkg_checksum),
        })
    }

    /// Downloads a QPKG from a URL, optionally verifies its checksum, then installs it to the cache.
    /// Centralizes the download+verify+install flow shared by qpackages restore, `qpm2 install --url`,
    /// and dependency-level `qpkgUrl` overrides.
    pub fn install_qpkg_from_url(
        &mut self,
        url: &str,
        checksum: Option<&str>,
        overwrite_existing: bool,
        version: Option<Version>,
    ) -> color_eyre::Result<Artifact> {
        println!("Downloading {}", url.file_path_color());
        let bytes = download_bytes(url)?;

        if let Some(checksum) = checksum {
            let result = Sha256::digest(&bytes);
            let hash_hex = hex::encode(result);

            if !hash_hex.eq_ignore_ascii_case(checksum) {
                bail!(
                    "Checksum mismatch for {}: expected {}, got {}",
                    url.blue(),
                    checksum,
                    hash_hex
                );
            }
        }

        let cursor = std::io::Cursor::new(bytes);
        self.install_qpkg(cursor, overwrite_existing, version)
    }

    pub fn copy_from_cache(
        &self,
        package: &PackageConfig,
        restored_deps: &[ResolvedDependency],
        workspace_dir: &Path,
    ) -> Result<()> {
        let files = self.collect_deps(package, restored_deps, workspace_dir)?;

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
                if !src.exists() {
                    bail!(
                        "The file or folder\n\t'{}'\ndid not exist! what happened to the cache? you should probably run {} to make sure everything is in order...",
                        src.display().bright_yellow(),
                        "qpm2 cache clear".bright_yellow()
                    );
                }

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
    pub fn collect_files_of_package(&self, package: &PackageConfig) -> Result<PackageFiles> {
        let cache_root = &self.root;
        let dep_cache_path =
            PackageIdPath::new(package.id.clone()).version(package.version.clone());

        if !dep_cache_path.base_path(&cache_root).exists() {
            bail!(
                "Missing cache for dependency {}:{}",
                package.id.dependency_id_color(),
                package.version.dependency_version_color()
            );
        }

        let headers_path = dep_cache_path.src_path(&cache_root);

        if !headers_path.exists() {
            bail!(
                "Missing src for dependency {}:{} at {}",
                package.id.dependency_id_color(),
                package.version.dependency_version_color(),
                headers_path.display().file_path_color()
            );
        }

        let expected_binaries = package.workspace.out_binaries.clone().unwrap_or_default();
        let binaries: Vec<PathBuf> = expected_binaries
            .iter()
            .map(|b| dep_cache_path.binary_path(&cache_root, b))
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
            headers: headers_path,
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
        &self,
        package: &PackageConfig,
        restored_deps: &[ResolvedDependency],
        workspace_dir: &Path,
    ) -> Result<HashMap<PathBuf, PathBuf>> {
        // validate exists dependencies
        let cache_root = &self.root;
        let missing_dependencies: Vec<_> = restored_deps
            .iter()
            .filter_map(|r| {
                let package_path = PackageIdPath::new(r.config.id.clone())
                    .version(r.config.version.clone())
                    .base_path(&cache_root);
                // if the package path does not exist, return the id and version
                package_path
                    .exists()
                    .not()
                    .then_some(format!("{}:{}", r.config.id, r.config.version))
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
                    self.collect_files_of_package(&resolved_dep.config)?;

                Ok((resolved_dep, collect_files_of_package))
            })
            .try_collect()?;

        let (direct_deps, indirect_deps): (Vec<_>, Vec<_>) =
            // partition by direct dependencies and indirect
            deps.into_iter().partition(|(unknown_dep, _)| {
                package
                    .dependencies
                    .iter()
                    .any(|direct_dep| *direct_dep.0 == unknown_dep.config.id)
            });

        // direct dependencies copy the binaries to the extern_binaries folder
        for (direct_dep, direct_dep_files) in &direct_deps {
            for binary in &direct_dep_files.binaries {
                let file_name = binary.file_name().expect("Failed to get file name");

                if !binary.exists() {
                    bail!(
                        "Missing binary {} for dependency {}:{}",
                        binary.display().bright_yellow(),
                        direct_dep.config.id.dependency_id_color(),
                        direct_dep.config.version.dependency_version_color()
                    );
                }

                // copy to extern/libs/{file_name}
                paths.insert(binary.clone(), extern_binaries.join(file_name));
            }
        }

        // Get headers of all dependencies restored
        for (dep, dep_files) in direct_deps.into_iter().chain(indirect_deps) {
            let project_deps_headers_target = extern_headers.join(dep.config.id.0.clone());

            let exposed_headers = dep_files.headers;
            if !exposed_headers.exists() {
                bail!(
                    "Missing header files {} for {}:{}",
                    exposed_headers.display().file_path_color(),
                    dep.config.id.dependency_id_color(),
                    dep.config.version.dependency_version_color()
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
        self.registry.artifacts.remove(package);
        let packages_path = PackageIdPath::new(package.clone()).versions_path(&self.root);
        if !packages_path.exists() {
            return Ok(());
        }
        std::fs::remove_dir_all(packages_path)?;
        Ok(())
    }
    pub fn remove_package(&mut self, package: &DependencyId, version: &Version) -> Result<()> {
        self.registry
            .artifacts
            .get_mut(package)
            .ok_or_eyre(format!("No package found {package}/{version}"))?
            .remove(version);

        let packages_path = PackageIdPath::new(package.clone())
            .version(version.clone())
            .base_path(&self.root);

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

    fn get_package(&self, id: &DependencyId, version: &Version) -> Result<Option<Artifact>> {
        Ok(self.get_artifact(id, version).cloned())
    }

    fn get_package_names(&self) -> Result<Vec<DependencyId>> {
        Ok(self.registry.artifacts.keys().cloned().collect())
    }

    fn add_to_db_cache(
        &mut self,
        config: PackageConfig,
        qpkg_checksum: Option<String>,
        permanent: bool,
    ) -> Result<()> {
        if !permanent {
            return Ok(());
        }

        // don't copy files to cache
        // don't overwrite cache with backend
        self.add_artifact_to_map(config, qpkg_checksum, false)?;
        Ok(())
    }

    fn download_to_cache(&mut self, config: &PackageConfig) -> Result<bool> {
        let exist_in_db = self.get_artifact(&config.id, &config.version).is_some();
        if !exist_in_db {
            return Ok(false);
        }

        let package_path = PackageIdPath::new(config.id.clone()).version(config.version.clone());
        let cache_root = &self.root;
        if !package_path.src_path(cache_root).exists() {
            return Ok(false);
        }

        let config = PackageConfig::read(package_path.qpm_json_dir(cache_root));

        Ok(exist_in_db && package_path.src_path(cache_root).exists() && config.is_ok())
    }

    fn write_repo(&self) -> Result<()> {
        self.write()
    }

    fn is_online(&self) -> bool {
        false
    }
}
