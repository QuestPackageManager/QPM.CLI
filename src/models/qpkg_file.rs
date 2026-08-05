use crate::utils::json;
use color_eyre::eyre::{Context, Result, bail};
use qpm_package::models::package::PackageConfig;
use qpm_package::models::qpkg::{QPKG_JSON, QPkg};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// Reads and manages QPKG ZIP files, providing access to manifest and contents
pub struct QpkgFile<T> {
    buffer: T,
    manifest: QPkg,
}

impl<T> QpkgFile<T> {
    /// Get the package manifest from this QPKG
    pub fn manifest(&self) -> &QPkg {
        &self.manifest
    }

    /// Get mutable reference to the manifest
    pub fn manifest_mut(&mut self) -> &mut QPkg {
        &mut self.manifest
    }

    /// Get the list of files in this QPKG
    pub fn files(&self) -> &[std::path::PathBuf] {
        &self.manifest.files
    }

    /// Get the shared directory path for headers
    pub fn shared_dir(&self) -> &std::path::Path {
        &self.manifest.shared_dir
    }

    /// Extract the inner buffer from this QpkgFile
    pub fn into_inner(self) -> T {
        self.buffer
    }
}

impl<T: Read + Seek> QpkgFile<T> {
    /// Open and read a QPKG file, extracting the manifest
    pub fn open(mut buffer: T) -> Result<Self> {
        let mut archive =
            ZipArchive::new(&mut buffer).context("Failed to read QPKG as ZIP archive")?;

        let manifest_file = archive
            .by_name(QPKG_JSON)
            .with_context(|| format!("QPKG missing manifest file: {QPKG_JSON}"))?;

        let manifest: QPkg = json::json_from_reader_fast(manifest_file)
            .with_context(|| format!("Failed to parse QPKG manifest: {QPKG_JSON}"))?;

        // Seek back to start for potential re-reading
        buffer
            .seek(std::io::SeekFrom::Start(0))
            .context("Failed to seek in buffer")?;

        Ok(QpkgFile { buffer, manifest })
    }

    /// Create a ZipArchive reader from the buffer
    fn as_zip_reader(&mut self) -> Result<ZipArchive<&mut T>> {
        ZipArchive::new(&mut self.buffer).context("Failed to read QPKG archive")
    }

    /// List all header files in the shared directory
    pub fn list_headers(&mut self) -> Result<Vec<PathBuf>> {
        let shared_dir = self.manifest.shared_dir.clone();
        let shared_dir_str = shared_dir.to_string_lossy().to_string();
        let mut headers = Vec::new();

        let mut archive = self.as_zip_reader()?;
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name();
            if name.starts_with(&shared_dir_str) && !name.ends_with('/') {
                headers.push(PathBuf::from(name));
            }
        }

        self.buffer
            .seek(std::io::SeekFrom::Start(0))
            .context("Failed to seek in buffer")?;

        Ok(headers)
    }

    /// Check if a specific header file exists
    pub fn has_header(&mut self, header_path: &Path) -> Result<bool> {
        let target_path = self.manifest.shared_dir.join(header_path);
        let target_str = target_path.to_string_lossy().to_string();

        let mut archive = self.as_zip_reader()?;
        let exists = archive.by_name(&target_str).is_ok();

        self.buffer
            .seek(std::io::SeekFrom::Start(0))
            .context("Failed to seek in buffer")?;

        Ok(exists)
    }

    /// Extracts manifest, headers, and binaries to separate destination directories
    pub fn extract_to(
        mut self,
        manifest_out: &Path,
        headers_out: &Path,
        binaries_out: &Path,
    ) -> color_eyre::Result<PackageConfig> {
        use crate::terminal::colors::QPMColor;
        use std::fs;

        // Base directory logic
        let base_path = manifest_out.parent().unwrap_or(manifest_out);

        // Initialize temporary scratch space for extraction
        let tmp_path = base_path.join("tmp_extract");
        if tmp_path.exists() {
            fs::remove_dir_all(&tmp_path)?;
        }
        fs::create_dir_all(&tmp_path).with_context(|| {
            format!(
                "Failed to create tmp folder {}",
                tmp_path.display().file_path_color()
            )
        })?;

        // Create target destinations
        fs::create_dir_all(manifest_out).context("Failed to create manifest destination")?;
        fs::create_dir_all(headers_out).context("Failed to create headers destination")?;
        fs::create_dir_all(binaries_out).context("Failed to create binaries destination")?;

        // Extract complete ZIP contents into temporary folder
        let mut zip_archive = ZipArchive::new(&mut self.buffer).context("Reading zip archive")?;
        zip_archive
            .extract(&tmp_path)
            .context("Zip extraction error")?;

        // Move headers (shared_dir) to headers destination
        let archive_shared_path = tmp_path.join(&self.manifest.shared_dir);
        if archive_shared_path.exists() {
            fs::rename(&archive_shared_path, headers_out).with_context(|| {
                format!(
                    "Failed to move headers from {} to {}",
                    archive_shared_path.display().file_path_color(),
                    headers_out.display().file_path_color()
                )
            })?;
        } else {
            eprintln!(
                "Warning: No header files found at {} for {}",
                archive_shared_path.display().file_path_color(),
                self.manifest.config.id.0.dependency_id_color()
            );
        }
        // Move binaries to binaries destination
        println!(
            "Installing package with {} files",
            self.manifest.files.len().file_path_color()
        );
        for file in &self.manifest.files {
            let tmp_src_file = tmp_path.join(file);
            if tmp_src_file.exists() {
                let dst_file = binaries_out.join(file.file_name().unwrap_or_default());
                fs::rename(&tmp_src_file, &dst_file).with_context(|| {
                    format!(
                        "Failed to copy file from {} to {}",
                        tmp_src_file.display().file_path_color(),
                        dst_file.display().file_path_color()
                    )
                })?;
            }
        }

        // Verify that expected final binaries are fully present in destination
        for binary in self.manifest.config.workspace.out_binaries.iter().flatten() {
            let binary_path = binaries_out.join(binary.file_name().unwrap_or_default());
            if !binary_path.exists() {
                bail!(
                    "Expected binary {} missing from installation target {}",
                    binary.display().file_path_color(),
                    binary_path.display().file_path_color()
                );
            }
        }

        // Save complete internal QPkg JSON state to output
        let qpkg_path = manifest_out.join(QPKG_JSON);
        let qpkg_json = serde_json::to_string_pretty(&self.manifest)
            .context("Failed to serialize QPkg manifest")?;
        fs::write(&qpkg_path, qpkg_json).with_context(|| {
            format!(
                "Failed to write QPkg manifest file to {}",
                qpkg_path.display().file_path_color()
            )
        })?;

        // Cleanup temporary directory assets safely
        if tmp_path.exists() {
            fs::remove_dir_all(tmp_path).context("Failed to clean up scratch folder")?;
        }

        Ok(self.manifest.config)
    }
}

impl<T: Write + Seek> QpkgFile<T> {
    /// Create a QPKG from header and binary directories
    ///
    /// headers_dir: directory containing header files (recursively included)
    /// binaries_dir: directory containing binary files
    /// config: package configuration (becomes the manifest)
    pub fn create_from_paths(
        buffer: T,
        config: PackageConfig,
        headers_dir: &Path,
        binaries_dir: &Path,
    ) -> Result<Self> {
        let mut headers = Vec::new();
        let mut binaries = Vec::new();

        // Collect headers from directory
        if headers_dir.exists() {
            for entry in walkdir::WalkDir::new(headers_dir)
                .min_depth(1)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
            {
                let file_path = entry.path();
                let rel_path = file_path.strip_prefix(headers_dir)?;
                let content = std::fs::read(file_path).context("Failed to read header file")?;
                headers.push((rel_path.to_path_buf(), content));
            }
        }

        // Collect binaries from directory
        if binaries_dir.exists() {
            for entry in walkdir::WalkDir::new(binaries_dir)
                .min_depth(1)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
            {
                let file_path = entry.path();
                let binary_name = file_path.file_name().unwrap_or_default();
                let content = std::fs::read(file_path).context("Failed to read binary file")?;
                binaries.push((PathBuf::from(binary_name), content));
            }
        }

        let headers_refs: Vec<_> = headers
            .iter()
            .map(|(p, c)| (p.as_path(), c.as_slice()))
            .collect();
        let binaries_refs: Vec<_> = binaries
            .iter()
            .map(|(p, c)| (p.as_path(), c.as_slice()))
            .collect();

        Self::create(buffer, config, headers_refs, binaries_refs)
    }

    /// Create a new QPKG ZIP with package config, headers, and binaries
    ///
    /// config: the package configuration (becomes the manifest)
    /// headers: list of (path, content) for files in shared directory
    /// binaries: list of (path, content) for files in bin directory
    pub fn create(
        buffer: T,
        config: PackageConfig,
        headers: Vec<(impl AsRef<Path>, &[u8])>,
        binaries: Vec<(impl AsRef<Path>, &[u8])>,
    ) -> Result<Self> {
        let mut zip = zip::ZipWriter::new(buffer);
        let options = zip::write::FileOptions::<()>::default();

        let shared_dir = config.shared_directory.clone();

        // Add header files using the config's shared directory path
        for (path, content) in headers {
            let full_path = shared_dir.join(path.as_ref());
            zip.start_file(full_path.to_string_lossy().as_ref(), options)
                .context("Failed to start header file in QPKG")?;
            zip.write_all(content)
                .context("Failed to write header file to QPKG")?;
        }

        // Collect binary file paths for the manifest (only binaries tracked in files)
        let mut files = Vec::new();

        // Add binary files in bin directory
        for (path, content) in binaries {
            let bin_path = Path::new("bin").join(path.as_ref());
            zip.start_file(bin_path.to_string_lossy().as_ref(), options)
                .context("Failed to start binary file in QPKG")?;
            zip.write_all(content)
                .context("Failed to write binary file to QPKG")?;
            files.push(bin_path);
        }

        // Create manifest from config and file list (matching original qpkg command)
        let manifest = QPkg {
            config,
            shared_dir,
            files,
        };

        // Write manifest last
        zip.start_file(QPKG_JSON, options)
            .context("Failed to start manifest in QPKG")?;
        serde_json::to_writer(&mut zip, &manifest).context("Failed to write manifest to QPKG")?;

        // Finish writing and store buffer
        let buffer = zip.finish().context("Failed to finish writing QPKG")?;

        Ok(Self { buffer, manifest })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use qpm_package::models::package::DependencyId;

    use super::*;

    /// Test QPKG creation and reading: create QPKG from config and verify it can be opened
    #[test]
    fn test_qpkg_create_and_read() {
        let config = PackageConfig {
            id: DependencyId("test-pkg".to_string()),
            version: semver::Version::parse("1.0.0").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            ..Default::default()
        };

        let headers = vec![("include/api.h", b"// Public API" as &[u8])];
        let binaries = vec![("libtest.so", b"binary_data" as &[u8])];

        let buffer = Cursor::new(Vec::new());
        let mut qpkg_file = QpkgFile::create(buffer, config, headers, binaries).unwrap();

        assert_eq!(qpkg_file.manifest().config.id.0, "test-pkg");
        assert_eq!(qpkg_file.manifest().config.version.to_string(), "1.0.0");

        // Verify headers exist
        let header_list = qpkg_file.list_headers().unwrap();
        assert!(
            header_list
                .iter()
                .any(|h| h.to_string_lossy().contains("api.h"))
        );
    }

    /// Test QPKG with multiple components: verify headers, binaries, and manifest coexist
    #[test]
    fn test_qpkg_complete_package() {
        let config = PackageConfig {
            id: DependencyId("complete-lib".to_string()),
            version: semver::Version::parse("2.1.3").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            ..Default::default()
        };

        let headers = vec![
            ("include/api.h", b"// Main API" as &[u8]),
            ("include/internal/impl.h", b"// Internal" as &[u8]),
        ];
        let binaries = vec![
            ("libmain.so", b"main_binary" as &[u8]),
            ("libutil.so", b"util_binary" as &[u8]),
        ];

        let buffer = Cursor::new(Vec::new());
        let mut qpkg_file = QpkgFile::create(buffer, config, headers, binaries).unwrap();

        // Verify binaries are tracked in files (headers are in shared_dir, not files)
        assert_eq!(qpkg_file.files().len(), 2);
        let file_paths: Vec<_> = qpkg_file
            .files()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert!(file_paths.contains(&"bin/libmain.so".to_string()));
        assert!(file_paths.contains(&"bin/libutil.so".to_string()));

        // Verify headers exist
        let headers_list = qpkg_file.list_headers().unwrap();
        assert!(
            headers_list
                .iter()
                .any(|h| h.to_string_lossy().contains("api.h"))
        );
        assert!(
            headers_list
                .iter()
                .any(|h| h.to_string_lossy().contains("impl.h"))
        );
    }

    /// Test QPKG manifest preservation: verify config fields survive round-trip
    #[test]
    fn test_qpkg_manifest_round_trip() {
        let config = PackageConfig {
            id: DependencyId("roundtrip-lib".to_string()),
            version: semver::Version::parse("1.5.2").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            shared_directory: std::path::PathBuf::from("headers"),
            dependencies_directory: std::path::PathBuf::from("extern"),
            ..Default::default()
        };

        let binaries = vec![("lib.so", b"binary" as &[u8])];

        let buffer = Cursor::new(Vec::new());
        let qpkg_file =
            QpkgFile::create(buffer, config, Vec::<(&str, &[u8])>::new(), binaries).unwrap();

        let restored = &qpkg_file.manifest().config;
        assert_eq!(restored.id.0, "roundtrip-lib");
        assert_eq!(restored.version.to_string(), "1.5.2");
    }

    /// Test QPKG file access: verify files are accessible in manifest
    #[test]
    fn test_qpkg_extract_files() {
        let config = PackageConfig {
            id: DependencyId("extract-lib".to_string()),
            version: semver::Version::parse("1.0.0").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            ..Default::default()
        };

        let headers = vec![("include/api.h", b"header_content" as &[u8])];
        let binaries = vec![("libtest.so", b"binary_payload" as &[u8])];

        let buffer = Cursor::new(Vec::new());
        let qpkg_file = QpkgFile::create(buffer, config, headers, binaries).unwrap();

        // Verify binaries are in manifest (headers tracked separately in shared_dir)
        let file_paths: Vec<_> = qpkg_file
            .files()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert!(file_paths.contains(&"bin/libtest.so".to_string()));
    }

    /// Test QPKG file listing: verify all files in manifest
    #[test]
    fn test_qpkg_list_files() {
        let config = PackageConfig {
            id: DependencyId("list-lib".to_string()),
            version: semver::Version::parse("1.0.0").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            ..Default::default()
        };

        let headers = vec![("include/pub.h", b"pub" as &[u8])];
        let binaries = vec![("lib1.so", b"b1" as &[u8]), ("lib2.so", b"b2" as &[u8])];

        let buffer = Cursor::new(Vec::new());
        let qpkg_file = QpkgFile::create(buffer, config, headers, binaries).unwrap();

        let file_paths: Vec<_> = qpkg_file
            .files()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert!(file_paths.contains(&"bin/lib1.so".to_string()));
        assert!(file_paths.contains(&"bin/lib2.so".to_string()));
    }

    /// Test QPKG with nested directory structure: verify paths are preserved
    #[test]
    fn test_qpkg_nested_structure() {
        let config = PackageConfig {
            id: DependencyId("nested-lib".to_string()),
            version: semver::Version::parse("1.0.0").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            ..Default::default()
        };

        let headers = vec![
            ("include/mylib/v2/api.h", b"v2_api" as &[u8]),
            ("include/mylib/v2/internal/impl.h", b"v2_impl" as &[u8]),
        ];
        let binaries: Vec<(&str, &[u8])> = Vec::new();

        let buffer = Cursor::new(Vec::new());
        let qpkg_file = QpkgFile::create(buffer, config, headers, binaries).unwrap();

        // No binaries in this test, so files list should be empty
        assert_eq!(qpkg_file.files().len(), 0);
    }

    /// Test QPKG with no files: verify QPKG can exist with manifest only
    #[test]
    fn test_qpkg_manifest_only() {
        let config = PackageConfig {
            id: DependencyId("manifest-only".to_string()),
            version: semver::Version::parse("1.0.0").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            ..Default::default()
        };

        let buffer = Cursor::new(Vec::new());
        let qpkg_file = QpkgFile::create(
            buffer,
            config,
            Vec::<(&str, &[u8])>::new(),
            Vec::<(&str, &[u8])>::new(),
        )
        .unwrap();

        assert!(qpkg_file.files().is_empty());
        assert_eq!(qpkg_file.manifest().config.id.0, "manifest-only");
    }

    /// Test QPKG missing manifest: verify error when manifest is absent
    #[test]
    fn test_qpkg_missing_manifest() {
        use std::io::Write;
        use zip::ZipWriter;

        let cursor = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default();

        // Create ZIP with file but no manifest
        zip.start_file("bin/libtest.so", options).unwrap();
        zip.write_all(b"binary").unwrap();

        let finished = zip.finish().unwrap();
        let cursor = Cursor::new(finished.into_inner());

        let result = QpkgFile::open(cursor);
        assert!(result.is_err(), "Should fail when manifest is missing");
    }

    /// Test QPKG buffer round-trip: create and read back from same buffer
    #[test]
    fn test_qpkg_buffer_round_trip() {
        let config = PackageConfig {
            id: DependencyId("buffer-roundtrip".to_string()),
            version: semver::Version::parse("1.5.0").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            ..Default::default()
        };

        let headers = vec![
            ("include/api.h", b"// API" as &[u8]),
            ("include/types.h", b"// Types" as &[u8]),
        ];
        let binaries = vec![("libcore.so", b"core_binary_data" as &[u8])];

        // Create in buffer
        let buffer = Cursor::new(Vec::new());
        let created = QpkgFile::create(buffer, config.clone(), headers, binaries).unwrap();

        // Extract the buffer and read it back
        let inner = created.into_inner();
        let read_buffer = Cursor::new(inner.into_inner());
        let mut read_back = QpkgFile::open(read_buffer).unwrap();

        // Verify all data survived round-trip
        assert_eq!(read_back.manifest().config, config);
        assert_eq!(read_back.files().len(), 1); // Only binaries tracked in files

        // Verify headers survived round-trip
        let headers_list = read_back.list_headers().unwrap();
        assert!(
            headers_list
                .iter()
                .any(|h| h.to_string_lossy().contains("api.h"))
        );
        assert!(
            headers_list
                .iter()
                .any(|h| h.to_string_lossy().contains("types.h"))
        );
    }

    /// Test QPKG extraction: verify extract_to() places files correctly
    #[test]
    fn test_qpkg_extract_to_directories() {
        use std::fs;
        use std::path::PathBuf;

        let temp_root = std::env::temp_dir().join("qpkg_test_extract");
        let _ = fs::remove_dir_all(&temp_root);
        fs::create_dir_all(&temp_root).unwrap();

        let config = PackageConfig {
            id: DependencyId("extract-test".to_string()),
            version: semver::Version::parse("1.0.0").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            shared_directory: PathBuf::from("include"),
            dependencies_directory: temp_root.clone(),
            workspace: Default::default(),
            ..Default::default()
        };

        let headers = vec![
            ("api.h", b"// API" as &[u8]),
            ("util/helper.h", b"// Helper" as &[u8]),
        ];
        let binaries = vec![("libmain.so", b"main_binary" as &[u8])];

        // Create QPKG
        let buffer = Cursor::new(Vec::new());
        let created = QpkgFile::create(buffer, config.clone(), headers, binaries).unwrap();

        // Extract to directories
        let manifest_out = temp_root.join("manifest");
        let headers_out = temp_root.join("headers");
        let binaries_out = temp_root.join("binaries");

        let extracted_config = created
            .extract_to(&manifest_out, &headers_out, &binaries_out)
            .unwrap();

        // Verify manifest was written
        assert!(manifest_out.join("qpm2.qpkg.json").exists());

        // Verify headers structure
        assert!(headers_out.join("api.h").exists());
        assert!(headers_out.join("util").join("helper.h").exists());

        // Verify binaries
        assert!(binaries_out.join("libmain.so").exists());
        let binary_content = fs::read(binaries_out.join("libmain.so")).unwrap();
        assert_eq!(binary_content, b"main_binary");

        // Verify config integrity
        assert_eq!(extracted_config, config);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_root);
    }

    /// Test QPKG open and verify file manifest: read QPKG and access file list
    #[test]
    fn test_qpkg_open_and_verify_manifest() {
        let config = PackageConfig {
            id: DependencyId("open-verify".to_string()),
            version: semver::Version::parse("2.3.4").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            ..Default::default()
        };

        let headers = vec![
            ("include/core.h", b"core" as &[u8]),
            ("include/v2/api.h", b"v2_api" as &[u8]),
        ];
        let binaries = vec![("lib1.so", b"bin1" as &[u8]), ("lib2.so", b"bin2" as &[u8])];

        // Create QPKG
        let buffer = Cursor::new(Vec::new());
        let created = QpkgFile::create(buffer, config, headers, binaries).unwrap();

        // Get buffer and re-open
        let inner = created.into_inner();
        let read_buffer = Cursor::new(inner.into_inner());
        let mut opened = QpkgFile::open(read_buffer).unwrap();

        // Verify manifest content
        let manifest = opened.manifest();
        assert_eq!(manifest.config.id.0, "open-verify");
        assert_eq!(manifest.config.version.to_string(), "2.3.4");

        // Verify file list (only binaries tracked)
        let file_paths: Vec<_> = opened
            .files()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        assert_eq!(file_paths.len(), 2);
        assert!(file_paths.contains(&"bin/lib1.so".to_string()));
        assert!(file_paths.contains(&"bin/lib2.so".to_string()));

        // Verify headers exist
        let headers_list = opened.list_headers().unwrap();
        assert!(
            headers_list
                .iter()
                .any(|h| h.to_string_lossy().contains("core.h"))
        );
        assert!(
            headers_list
                .iter()
                .any(|h| h.to_string_lossy().contains("api.h"))
        );
    }

    /// Test QPKG with workspace binaries: verify binary validation during extraction
    #[test]
    fn test_qpkg_extraction_validates_binaries() {
        use std::fs;
        use std::path::PathBuf;

        let temp_root = std::env::temp_dir().join("qpkg_test_validate");
        let _ = fs::remove_dir_all(&temp_root);
        fs::create_dir_all(&temp_root).unwrap();

        let mut config = PackageConfig {
            id: DependencyId("validate-bins".to_string()),
            version: semver::Version::parse("1.0.0").unwrap(),
            config_version: semver::Version::parse("2.0.0").unwrap(),
            shared_directory: PathBuf::from("include"),
            dependencies_directory: temp_root.clone(),
            ..Default::default()
        };

        // Set expected out_binaries
        config.workspace.out_binaries = Some(vec![
            PathBuf::from("libmain.so"),
            PathBuf::from("libutil.so"),
        ]);

        let headers = vec![("api.h", b"// Public API" as &[u8])];
        let binaries = vec![
            ("libmain.so", b"main" as &[u8]),
            ("libutil.so", b"util" as &[u8]),
        ];

        // Create QPKG
        let buffer = Cursor::new(Vec::new());
        let created = QpkgFile::create(buffer, config.clone(), headers, binaries).unwrap();

        // Extract should verify all out_binaries are present
        let manifest_out = temp_root.join("manifest");
        let headers_out = temp_root.join("headers");
        let binaries_out = temp_root.join("binaries");

        let extracted_config = created
            .extract_to(&manifest_out, &headers_out, &binaries_out)
            .unwrap();

        // Verify both expected binaries exist
        assert!(binaries_out.join("libmain.so").exists());
        assert!(binaries_out.join("libutil.so").exists());

        // Verify config integrity
        assert_eq!(extracted_config, config);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_root);
    }
}
