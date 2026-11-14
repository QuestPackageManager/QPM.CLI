#[path = "support/mod.rs"]
mod support;

use std::collections::HashMap;

use qpm_cli::models::package::{PackageConfigExtensions, SharedPackageConfigExtensions};
use qpm_package::models::{
    package::{DependencyId, PackageConfig},
    shared_package::{SharedDependencyInfo, SharedPackageConfig},
};
use semver::Version;
use support::locked;
use tempfile::tempdir;

/// Regression: `exists` used to look for qpm2.json next to the directory instead of inside it
#[test]
fn exists_looks_inside_the_given_directory() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("workspace");
    std::fs::create_dir(&dir).unwrap();

    assert!(!PackageConfig::exists(&dir));
    PackageConfig::default().write(&dir).unwrap();
    assert!(PackageConfig::exists(&dir));

    // a qpm2.json in the parent must not count
    let sibling = parent.path().join("sibling");
    std::fs::create_dir(&sibling).unwrap();
    assert!(!PackageConfig::exists(&sibling));
}

fn restored(env: &[(&str, &str)]) -> SharedDependencyInfo {
    SharedDependencyInfo {
        restored_env: env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..locked(Version::new(1, 0, 0))
    }
}

#[test]
fn get_env_merges_dependencies_and_local_env_wins() {
    let shared = SharedPackageConfig {
        config: PackageConfig::default(),
        restored_dependencies: HashMap::from([
            (
                DependencyId("a".into()),
                restored(&[("FROM_A", "a"), ("OVERRIDDEN", "a")]),
            ),
            (DependencyId("b".into()), restored(&[("FROM_B", "b")])),
        ]),
        env: HashMap::from([
            ("OVERRIDDEN".to_string(), "local".to_string()),
            ("LOCAL".to_string(), "local".to_string()),
        ]),
    };

    let env = shared.get_env();

    assert_eq!(
        env,
        HashMap::from([
            ("FROM_A".to_string(), "a".to_string()),
            ("FROM_B".to_string(), "b".to_string()),
            ("OVERRIDDEN".to_string(), "local".to_string()),
            ("LOCAL".to_string(), "local".to_string()),
        ])
    );
}
