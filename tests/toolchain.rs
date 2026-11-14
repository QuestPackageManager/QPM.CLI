//! `write_toolchain_file`: dependency include paths are rebased into `extern/includes/<id>`,
//! compile flags from every dependency are combined, and each dependency's binaries are
//! listed at their `extern/libs` location.

#[path = "support/mod.rs"]
mod support;

use std::{collections::HashMap, fs::File, io::Cursor, path::PathBuf};

use qpm_cli::{
    models::toolchain::{ToolchainData, write_toolchain_file},
    repository::file::FileRepository,
};
use qpm_package::models::{
    extra::PackageCompileOptions,
    package::PackageConfig,
    shared_package::SharedPackageConfig,
};
use semver::Version;
use support::{depend_on, id, locked, package, qpkg_bytes};
use tempfile::tempdir;

fn with_options(mut config: PackageConfig, include: &str, flag: &str) -> PackageConfig {
    config.compile_options = Some(PackageCompileOptions {
        include_paths: Some(vec![include.into()]),
        system_includes: Some(vec![format!("{include}/sys")]),
        cpp_flags: Some(vec![flag.into()]),
        c_flags: None,
    });
    config
}

#[test]
fn toolchain_rebases_includes_and_lists_binaries() {
    let cache = tempdir().unwrap();
    let out = tempdir().unwrap();
    let mut repo = FileRepository::read(cache.path().to_path_buf()).unwrap();

    let a = with_options(
        package("a", Version::new(1, 0, 0), Some(&["liba.so", "liba.a"])),
        "include",
        "-DA",
    );
    let b = with_options(package("b", Version::new(2, 0, 0), None), "src", "-DB");
    for dep in [&a, &b] {
        repo.install_qpkg(Cursor::new(qpkg_bytes(dep)), false, None)
            .unwrap();
    }

    let mut root = package("root", Version::new(0, 1, 0), None);
    depend_on(&mut root, "a", "^1");
    depend_on(&mut root, "b", "^2");
    let shared = SharedPackageConfig {
        config: root,
        restored_dependencies: HashMap::from([
            (id("a"), locked(a.version.clone())),
            (id("b"), locked(b.version.clone())),
        ]),
        env: Default::default(),
    };

    let path = out.path().join("toolchain.json");
    write_toolchain_file(&shared, &repo, &path, &repo).unwrap();
    let toolchain: ToolchainData = serde_json::from_reader(File::open(&path).unwrap()).unwrap();

    let sorted = |v: Option<Vec<String>>| {
        let mut v = v.unwrap_or_default();
        v.sort();
        v
    };
    let options = toolchain.compile_options;
    assert_eq!(
        sorted(options.include_paths),
        vec!["extern/includes/a/include", "extern/includes/b/src"]
    );
    assert_eq!(
        sorted(options.system_includes),
        vec!["extern/includes/a/include/sys", "extern/includes/b/src/sys"]
    );
    assert_eq!(sorted(options.cpp_flags), vec!["-DA", "-DB"]);

    let mut a_bins = toolchain.linked_binaries[&id("a")].clone();
    a_bins.sort();
    assert_eq!(
        a_bins,
        vec![
            PathBuf::from("extern/libs/liba.a"),
            PathBuf::from("extern/libs/liba.so")
        ]
    );
    assert!(toolchain.linked_binaries[&id("b")].is_empty());

    assert_eq!(toolchain.package_id, "root");
    assert_eq!(toolchain.include_dir, PathBuf::from("extern/includes"));
    assert_eq!(toolchain.libs_dir, PathBuf::from("extern/libs"));
}
