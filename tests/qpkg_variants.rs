#![cfg(feature = "cli")]

//! The QPKG lifecycle for each kind of package (header-only, shared, static, mixed), driven
//! through the real CLI with no network: `qpkg --no-build` → `install --path` →
//! `restore --offline` in a project that depends on it.

#[path = "support/mod.rs"]
mod support;

use std::{fs, fs::File, io::Read, path::Path};

use qpm_cli::models::{
    package::PackageConfigExtensions, package_files::PackageVersionPath, qpkg_file::QpkgFile,
    toolchain::ToolchainData,
};
use qpm_package::models::package::PackageConfig;
use qpm_package::models::workspace::HeaderExportConfig;
use semver::Version;
use support::{depend_on, file_names, header_name, id, package, run_qpm, user_config, workspace};
use tempfile::tempdir;

/// Producer workspace for `config`: its header in `shared/` and a fake file for each
/// `out_binaries` entry in `extern/build/`, ready for `qpkg --no-build`.
fn producer(parent: &Path, config: &PackageConfig) -> std::path::PathBuf {
    let dir = workspace(parent, config);
    fs::create_dir_all(dir.join("shared")).unwrap();
    fs::write(dir.join("shared").join(header_name(config)), "// header\n").unwrap();

    let build = dir.join("extern/build");
    fs::create_dir_all(&build).unwrap();
    for binary in config.workspace.out_binaries.iter().flatten() {
        fs::write(build.join(binary), format!("binary {}", binary.display())).unwrap();
    }
    dir
}

/// Packages `config` with `qpm2 qpkg --no-build`, checks the archive's contents, then
/// installs it into the cache with `qpm2 install --path` and checks the cache layout
fn package_and_install(root: &Path, cache: &Path, config: &PackageConfig) {
    let user = user_config(cache);
    let dir = producer(root, config);

    run_qpm(&user, &dir, &["qpkg", "--no-build"]).expect("qpkg --no-build should succeed");

    let qpkg_path = dir.join(format!("{}.qpkg", config.id.0));
    let mut qpkg = QpkgFile::open(File::open(&qpkg_path).unwrap()).unwrap();
    assert_eq!(qpkg.manifest().config, *config);
    let headers: Vec<String> = qpkg
        .list_headers()
        .unwrap()
        .iter()
        .map(|h| h.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(headers, vec![header_name(config)]);
    let mut files: Vec<String> = qpkg
        .files()
        .iter()
        .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    files.sort();
    let mut expected: Vec<String> = binaries_of(config);
    expected.sort();
    assert_eq!(
        files, expected,
        "the qpkg should hold exactly the out_binaries"
    );

    run_qpm(
        &user,
        &dir,
        &["install", "--path", qpkg_path.to_str().unwrap()],
    )
    .expect("installing the qpkg should succeed");

    let cached = PackageVersionPath::new(config.id.clone(), config.version.clone());
    assert!(
        cached.src_path(cache).join(header_name(config)).exists(),
        "header should be cached"
    );
    assert_eq!(file_names(&cached.binaries_path(cache)), {
        let mut b = binaries_of(config);
        b.sort();
        b
    });
}

fn binaries_of(config: &PackageConfig) -> Vec<String> {
    config
        .workspace
        .out_binaries
        .iter()
        .flatten()
        .map(|b| b.to_string_lossy().into_owned())
        .collect()
}

/// A consumer depending on each of `deps` (at `^version`), restored offline with a toolchain
/// file. Returns its workspace dir.
fn restore_consumer(root: &Path, cache: &Path, deps: &[&PackageConfig]) -> std::path::PathBuf {
    let mut consumer = package("consumer", Version::new(1, 0, 0), None);
    consumer.workspace.toolchain_out = Some("toolchain.json".into());
    for dep in deps {
        depend_on(&mut consumer, &dep.id.0, &format!("^{}", dep.version));
    }
    let dir = workspace(root, &consumer);

    run_qpm(&user_config(cache), &dir, &["restore", "--offline"])
        .expect("offline restore should succeed");
    dir
}

fn toolchain(consumer: &Path) -> ToolchainData {
    serde_json::from_reader(File::open(consumer.join("toolchain.json")).unwrap()).unwrap()
}

/// Runs the whole lifecycle for one directly depended-on package and checks what landed in
/// the consumer's `extern/`
fn assert_lifecycle(config: PackageConfig) {
    let root = tempdir().unwrap();
    let cache = tempdir().unwrap();

    package_and_install(root.path(), cache.path(), &config);
    let consumer = restore_consumer(root.path(), cache.path(), &[&config]);

    assert!(
        consumer
            .join("extern/includes")
            .join(&config.id.0)
            .join(header_name(&config))
            .exists(),
        "header should be restored into extern/includes/<id>"
    );

    // restore always creates extern/libs, even when nothing goes in it (header-only)
    assert!(
        consumer.join("extern/libs").is_dir(),
        "restore should create extern/libs"
    );
    let mut expected = binaries_of(&config);
    expected.sort();
    assert_eq!(file_names(&consumer.join("extern/libs")), expected);

    let linked = toolchain(&consumer)
        .linked_binaries
        .remove(&config.id)
        .expect("every restored dependency is listed in linkedBinaries");
    let mut linked: Vec<String> = linked
        .iter()
        .map(|p| {
            assert!(
                p.starts_with("extern/libs"),
                "{p:?} should point into extern/libs"
            );
            p.file_name().unwrap().to_string_lossy().into_owned()
        })
        .collect();
    linked.sort();
    assert_eq!(linked, expected);
}

#[test]
fn header_only_without_out_binaries() {
    assert_lifecycle(package("headeronly", Version::new(1, 0, 0), None));
}

/// `Some([])` must behave exactly like `None`
#[test]
fn header_only_with_empty_out_binaries() {
    assert_lifecycle(package("headeronly", Version::new(1, 0, 0), Some(&[])));
}

#[test]
fn shared_library() {
    assert_lifecycle(package(
        "sharedlib",
        Version::new(1, 0, 0),
        Some(&["libsharedlib.so"]),
    ));
}

#[test]
fn checkout_binary_and_selected_headers_restore_with_a_prefix() {
    let root = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let mut config = package(
        "layout",
        Version::new(1, 0, 0),
        Some(&["build/arm64-v8a/liblayout.so"]),
    );
    config.workspace.out_binaries_directory = Some(".".into());
    config.workspace.header_export = Some(HeaderExportConfig {
        prefix: Some("shared".into()),
        paths: Some(vec!["api.hpp".into(), "vendor/source".into()]),
    });
    let dir = workspace(root.path(), &config);
    fs::create_dir_all(dir.join("shared/vendor/source")).unwrap();
    fs::write(dir.join("shared/api.hpp"), "// public header").unwrap();
    fs::write(
        dir.join("shared/vendor/source/nested.hpp"),
        "// vendor header",
    )
    .unwrap();
    fs::write(dir.join("shared/vendor/.git"), "not a public file").unwrap();
    fs::create_dir_all(dir.join("build/arm64-v8a")).unwrap();
    fs::write(dir.join("build/arm64-v8a/liblayout.so"), b"checkout binary").unwrap();

    let user = user_config(cache.path());
    run_qpm(&user, &dir, &["qpkg", "--no-build"]).unwrap();
    let qpkg = dir.join("layout.qpkg");
    run_qpm(&user, &dir, &["install", "--path", qpkg.to_str().unwrap()]).unwrap();
    let consumer = restore_consumer(root.path(), cache.path(), &[&config]);
    let include = consumer.join("extern/includes/layout");
    assert!(include.join("shared/api.hpp").is_file());
    assert!(include.join("shared/vendor/source/nested.hpp").is_file());
    assert!(!include.join("api.hpp").exists());
    assert!(!include.join("shared/vendor/.git").exists());
    assert_eq!(
        fs::read(consumer.join("extern/libs/liblayout.so")).unwrap(),
        b"checkout binary"
    );

    fs::create_dir_all(dir.join("alternate/build/arm64-v8a")).unwrap();
    fs::write(
        dir.join("alternate/build/arm64-v8a/liblayout.so"),
        b"override binary",
    )
    .unwrap();
    run_qpm(
        &user,
        &dir,
        &[
            "qpkg",
            "--no-build",
            "--input-bins",
            "alternate",
            "override.qpkg",
        ],
    )
    .unwrap();
    let mut zip = zip::ZipArchive::new(File::open(dir.join("override.qpkg")).unwrap()).unwrap();
    let mut bytes = Vec::new();
    zip.by_name("bin/liblayout.so")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes, b"override binary");
}

#[test]
fn static_library() {
    assert_lifecycle(package(
        "staticlib",
        Version::new(1, 0, 0),
        Some(&["libstaticlib.a"]),
    ));
}

#[test]
fn shared_and_static_library() {
    assert_lifecycle(package(
        "mixedlib",
        Version::new(1, 0, 0),
        Some(&["libmixedlib.so", "libmixedlib_static.a"]),
    ));
}

/// consumer → a (shared) → b (static): every package's headers are restored, but only direct
/// dependencies' binaries are copied into extern/libs
#[test]
fn transitive_dependency_gets_headers_but_not_binaries() {
    let root = tempdir().unwrap();
    let cache = tempdir().unwrap();

    let b = package("b", Version::new(1, 0, 0), Some(&["libb.a"]));
    let mut a = package("a", Version::new(1, 0, 0), Some(&["liba.so"]));
    depend_on(&mut a, "b", "^1.0.0");

    package_and_install(root.path(), cache.path(), &b);
    package_and_install(root.path(), cache.path(), &a);
    let consumer = restore_consumer(root.path(), cache.path(), &[&a]);

    for dep in [&a, &b] {
        assert!(
            consumer
                .join("extern/includes")
                .join(&dep.id.0)
                .join(header_name(dep))
                .exists(),
            "{} headers should be restored",
            dep.id
        );
    }
    assert_eq!(file_names(&consumer.join("extern/libs")), vec!["liba.so"]);

    let shared = qpm_package::models::shared_package::SharedPackageConfig::read(&consumer).unwrap();
    assert!(shared.restored_dependencies.contains_key(&id("b")));
}

/// A listed binary that was never built must fail packaging, not produce a qpkg without it
#[test]
fn qpkg_fails_when_an_out_binary_is_missing() {
    let root = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let config = package("broken", Version::new(1, 0, 0), Some(&["libbroken.so"]));

    let dir = producer(root.path(), &config);
    fs::remove_file(dir.join("extern/build/libbroken.so")).unwrap();

    let err = run_qpm(&user_config(cache.path()), &dir, &["qpkg", "--no-build"])
        .expect_err("packaging without the binary should fail");
    assert!(
        format!("{err:?}").contains("libbroken.so"),
        "error should name the missing binary: {err:?}"
    );
    assert!(!dir.join("broken.qpkg").exists());
}

/// A dev dependency is direct too: its binaries must be copied into extern/libs
#[test]
fn dev_dependency_binaries_are_copied() {
    let root = tempdir().unwrap();
    let cache = tempdir().unwrap();

    let lib = package("devlib", Version::new(1, 0, 0), Some(&["libdevlib.a"]));
    package_and_install(root.path(), cache.path(), &lib);

    let mut consumer = package("consumer", Version::new(1, 0, 0), None);
    depend_on(&mut consumer, "devlib", "^1.0.0");
    consumer.dev_dependencies = std::mem::take(&mut consumer.dependencies);
    let dir = workspace(root.path(), &consumer);

    run_qpm(&user_config(cache.path()), &dir, &["restore", "--offline"])
        .expect("offline restore should succeed");

    assert_eq!(file_names(&dir.join("extern/libs")), vec!["libdevlib.a"]);
}
