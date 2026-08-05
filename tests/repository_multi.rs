mod mocks;

use std::path::Path;

use qpm_cli::repository::{Repository, multi::MultiDependencyRepository};
use qpm_package::models::package::{DependencyId, PackageConfig};
use semver::Version;

use mocks::mock_repository::MockRepository;

fn pkg(id: &str, version: Version) -> PackageConfig {
    PackageConfig {
        id: DependencyId(id.to_string()),
        version,
        ..Default::default()
    }
}

/// Versions available across all repositories should be merged, deduplicated, and sorted
/// descending - matching the `Repository::get_package_versions` contract ("Ordered by
/// version descending").
#[test]
fn get_package_versions_merges_dedupes_and_sorts_descending() {
    let id = DependencyId("pkg".to_string());
    let a = MockRepository::new(false)
        .with_package(pkg("pkg", Version::new(1, 0, 0)))
        .with_package(pkg("pkg", Version::new(2, 0, 0)));
    let b = MockRepository::new(false)
        // 2.0.0 present in both repos - must not appear twice in the merged result
        .with_package(pkg("pkg", Version::new(2, 0, 0)))
        .with_package(pkg("pkg", Version::new(3, 0, 0)));

    let multi = MultiDependencyRepository::new(vec![Box::new(a), Box::new(b)]);

    let versions = multi.get_package_versions(&id).unwrap().unwrap();

    assert_eq!(
        versions,
        vec![
            Version::new(3, 0, 0),
            Version::new(2, 0, 0),
            Version::new(1, 0, 0),
        ]
    );
}

/// A package unknown to every repository should resolve to `None`, not an empty list.
#[test]
fn get_package_versions_none_when_unknown_everywhere() {
    let multi = MultiDependencyRepository::new(vec![
        Box::new(MockRepository::new(false)),
        Box::new(MockRepository::new(false)),
    ]);

    let versions = multi
        .get_package_versions(&DependencyId("missing".to_string()))
        .unwrap();

    assert_eq!(versions, None);
}

/// When the same id:version exists in more than one repository, the first repository in the
/// list that has it wins (repositories are meant to be checked "in order").
#[test]
fn get_package_returns_from_first_repo_that_has_it() {
    let id = DependencyId("pkg".to_string());
    let version = Version::new(1, 0, 0);

    let mut first_config = pkg("pkg", version.clone());
    first_config.shared_directory = "from-first-repo".into();
    let mut second_config = pkg("pkg", version.clone());
    second_config.shared_directory = "from-second-repo".into();

    let first = MockRepository::new(false).with_package(first_config);
    let second = MockRepository::new(false).with_package(second_config);

    let multi = MultiDependencyRepository::new(vec![Box::new(first), Box::new(second)]);

    let result = multi.get_package(&id, &version).unwrap().unwrap();
    assert_eq!(result.config.shared_directory, Path::new("from-first-repo"));
}

/// Package names should be deduplicated across repositories that share a package.
#[test]
fn get_package_names_dedupes_across_repos() {
    let a = MockRepository::new(false).with_package(pkg("shared", Version::new(1, 0, 0)));
    let b = MockRepository::new(false)
        .with_package(pkg("shared", Version::new(1, 0, 0)))
        .with_package(pkg("only-in-b", Version::new(1, 0, 0)));

    let multi = MultiDependencyRepository::new(vec![Box::new(a), Box::new(b)]);

    let mut names = multi.get_package_names().unwrap();
    names.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(
        names,
        vec![
            DependencyId("only-in-b".to_string()),
            DependencyId("shared".to_string()),
        ]
    );
}

/// The combined repository should report online if *any* underlying repository is online,
/// even if others are offline-only (e.g. a local file cache alongside a remote backend).
#[test]
fn is_online_if_any_repo_is_online() {
    let all_offline = MultiDependencyRepository::new(vec![
        Box::new(MockRepository::new(false)),
        Box::new(MockRepository::new(false)),
    ]);
    assert!(!all_offline.is_online());

    let one_online = MultiDependencyRepository::new(vec![
        Box::new(MockRepository::new(false)),
        Box::new(MockRepository::new(true)),
    ]);
    assert!(one_online.is_online());
}

/// Downloading should be delegated to whichever repository actually has the package, not
/// broadcast to every repository.
#[test]
fn download_to_cache_delegates_to_the_repo_that_has_the_package() {
    let id = DependencyId("pkg".to_string());
    let version = Version::new(1, 0, 0);

    let without = MockRepository::new(false);
    let with = MockRepository::new(false).with_package(pkg("pkg", version.clone()));

    let with_handle = with.clone();
    let without_handle = without.clone();

    let mut multi = MultiDependencyRepository::new(vec![Box::new(without), Box::new(with)]);

    let config = pkg("pkg", version.clone());
    let downloaded = multi.download_to_cache(&config).unwrap();

    assert!(downloaded);
    assert_eq!(with_handle.downloaded(), vec![(id, version)]);
    assert!(without_handle.downloaded().is_empty());
}

/// If no repository has the package at all, downloading should fail loudly rather than
/// silently doing nothing.
#[test]
fn download_to_cache_errors_when_no_repo_has_the_package() {
    let mut multi = MultiDependencyRepository::new(vec![
        Box::new(MockRepository::new(false)),
        Box::new(MockRepository::new(false)),
    ]);

    let config = pkg("missing", Version::new(1, 0, 0));
    assert!(multi.download_to_cache(&config).is_err());
}

/// Registering a package in the shared/db cache should propagate to every underlying
/// repository, not just the first one.
#[test]
fn add_to_db_cache_forwards_to_every_repo() {
    let a = MockRepository::new(false);
    let b = MockRepository::new(false);
    let a_handle = a.clone();
    let b_handle = b.clone();

    let mut multi = MultiDependencyRepository::new(vec![Box::new(a), Box::new(b)]);

    let id = DependencyId("pkg".to_string());
    let version = Version::new(1, 0, 0);
    multi
        .add_to_db_cache(pkg("pkg", version.clone()), None, true)
        .unwrap();

    assert!(a_handle.get_package(&id, &version).unwrap().is_some());
    assert!(b_handle.get_package(&id, &version).unwrap().is_some());
}
