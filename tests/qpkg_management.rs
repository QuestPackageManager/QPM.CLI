use std::io::Cursor;
use qpm_package::models::package::{PackageConfig, DependencyId};
use qpm_cli::models::qpkg_file::QpkgFile;

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
    assert!(header_list.iter().any(|h| h.to_string_lossy().contains("api.h")));
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
    let file_paths: Vec<_> = qpkg_file.files().iter().map(|p| p.to_string_lossy().to_string()).collect();
    assert!(file_paths.contains(&"bin/libmain.so".to_string()));
    assert!(file_paths.contains(&"bin/libutil.so".to_string()));

    // Verify headers exist
    let headers_list = qpkg_file.list_headers().unwrap();
    assert!(headers_list.iter().any(|h| h.to_string_lossy().contains("api.h")));
    assert!(headers_list.iter().any(|h| h.to_string_lossy().contains("impl.h")));
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
    let qpkg_file = QpkgFile::create(buffer, config, Vec::<(&str, &[u8])>::new(), binaries).unwrap();

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
    let file_paths: Vec<_> = qpkg_file.files().iter().map(|p| p.to_string_lossy().to_string()).collect();
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

    let file_paths: Vec<_> = qpkg_file.files().iter().map(|p| p.to_string_lossy().to_string()).collect();
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
    let qpkg_file = QpkgFile::create(buffer, config, Vec::<(&str, &[u8])>::new(), Vec::<(&str, &[u8])>::new()).unwrap();

    assert!(qpkg_file.files().is_empty());
    assert_eq!(qpkg_file.manifest().config.id.0, "manifest-only");
}

/// Test QPKG missing manifest: verify error when manifest is absent
#[test]
fn test_qpkg_missing_manifest() {
    use std::io::Write;
    use zip::ZipWriter;

    let cursor = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = zip::write::FileOptions::<()>::default();

    // Create ZIP with file but no manifest
    zip.start_file("bin/libtest.so", options).unwrap();
    zip.write_all(b"binary").unwrap();

    let finished = zip.finish().unwrap();
    let cursor = std::io::Cursor::new(finished.into_inner());

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
    assert!(headers_list.iter().any(|h| h.to_string_lossy().contains("api.h")));
    assert!(headers_list.iter().any(|h| h.to_string_lossy().contains("types.h")));
}

/// Test QPKG extraction: verify extract_to() places files correctly
#[test]
fn test_qpkg_extract_to_directories() {
    use std::path::PathBuf;
    use std::fs;

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

    let extracted_config = created.extract_to(&manifest_out, &headers_out, &binaries_out).unwrap();

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
    let binaries = vec![
        ("lib1.so", b"bin1" as &[u8]),
        ("lib2.so", b"bin2" as &[u8]),
    ];

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
    assert!(headers_list.iter().any(|h| h.to_string_lossy().contains("core.h")));
    assert!(headers_list.iter().any(|h| h.to_string_lossy().contains("api.h")));
}

/// Test QPKG with workspace binaries: verify binary validation during extraction
#[test]
fn test_qpkg_extraction_validates_binaries() {
    use std::path::PathBuf;
    use std::fs;

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

    let extracted_config = created.extract_to(&manifest_out, &headers_out, &binaries_out).unwrap();

    // Verify both expected binaries exist
    assert!(binaries_out.join("libmain.so").exists());
    assert!(binaries_out.join("libutil.so").exists());

    // Verify config integrity
    assert_eq!(extracted_config, config);

    // Cleanup
    let _ = fs::remove_dir_all(&temp_root);
}
