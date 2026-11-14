//! `MemcachedRepository` serves repeat lookups from memory, which is what keeps pubgrub's many
//! repeated queries cheap. These prove it caches (by mutating the inner repository behind it)
//! and that it doesn't cache misses.

mod mocks;
#[path = "support/mod.rs"]
mod support;

use mocks::mock_repository::MockRepository;
use qpm_cli::repository::{Artifact, Repository, memcached::MemcachedRepository};
use semver::Version;
use support::{id, package, test_agent};

#[test]
fn versions_are_served_from_cache_after_first_lookup() {
    let inner = MockRepository::new(false).with_package(package("a", Version::new(1, 0, 0), None));
    let repo = MemcachedRepository::new(inner.clone());

    let first = repo.get_package_versions(&id("a")).unwrap().unwrap();
    inner.insert(package("a", Version::new(2, 0, 0), None));
    let second = repo.get_package_versions(&id("a")).unwrap().unwrap();

    assert_eq!(&*first, &[Version::new(1, 0, 0)]);
    assert_eq!(&*second, &*first, "the new version must not be visible yet");
}

#[test]
fn missing_versions_are_not_cached() {
    let inner = MockRepository::new(false);
    let repo = MemcachedRepository::new(inner.clone());

    assert!(repo.get_package_versions(&id("late")).unwrap().is_none());
    inner.insert(package("late", Version::new(1, 0, 0), None));

    let versions = repo.get_package_versions(&id("late")).unwrap();
    assert_eq!(versions.as_deref(), Some(&[Version::new(1, 0, 0)][..]));
}

#[test]
fn package_is_served_from_cache_after_first_lookup() {
    let inner = MockRepository::new(false).with_package(package("a", Version::new(1, 0, 0), None));
    let repo = MemcachedRepository::new(inner.clone());

    let first = repo.get_package(&id("a"), &Version::new(1, 0, 0)).unwrap();

    let mut changed = package("a", Version::new(1, 0, 0), None);
    changed.shared_directory = "changed".into();
    inner.insert(changed);

    let second = repo.get_package(&id("a"), &Version::new(1, 0, 0)).unwrap();
    assert_eq!(first, second);
    assert_ne!(second.unwrap().config.shared_directory, std::path::Path::new("changed"));
}

#[test]
fn missing_package_is_not_cached() {
    let inner = MockRepository::new(false);
    let repo = MemcachedRepository::new(inner.clone());
    let v = Version::new(1, 0, 0);

    assert!(repo.get_package(&id("late"), &v).unwrap().is_none());
    inner.insert(package("late", v.clone(), None));
    assert!(repo.get_package(&id("late"), &v).unwrap().is_some());
}

#[test]
fn package_names_are_served_from_cache_after_first_lookup() {
    let inner = MockRepository::new(false).with_package(package("a", Version::new(1, 0, 0), None));
    let repo = MemcachedRepository::new(inner.clone());

    let first = repo.get_package_names().unwrap();
    inner.insert(package("b", Version::new(1, 0, 0), None));

    assert_eq!(repo.get_package_names().unwrap(), first);
    assert_eq!(first, vec![id("a")]);
}

#[test]
fn download_and_online_delegate_to_inner() {
    let inner = MockRepository::new(true).with_package(package("a", Version::new(1, 0, 0), None));
    let mut repo = MemcachedRepository::new(inner.clone());
    let agent = test_agent();

    let artifact = Artifact {
        config: package("a", Version::new(1, 0, 0), None),
        ..Default::default()
    };
    assert!(repo.download_to_cache(&agent, &artifact).unwrap());
    assert_eq!(inner.downloaded(), vec![(id("a"), Version::new(1, 0, 0))]);
    assert!(repo.is_online());
}
