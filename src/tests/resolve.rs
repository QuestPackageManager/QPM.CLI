use color_eyre::Result;
use itertools::Itertools;
use qpm_package::models::{package::DependencyId, shared_package::SharedPackageConfig};
use semver::Version;

use crate::{repository::Repository, resolver::dependency};

use super::mocks::repo::get_mock_repository;

#[test]
fn get_artifact_names() -> Result<()> {
    let repo = get_mock_repository();
    let names = repo.get_package_names()?;

    let packages_name_mapped = repo.artifacts.keys().cloned().collect_vec();

    assert_eq!(names.len(), packages_name_mapped.len());
    assert_eq!(names, packages_name_mapped);
    Ok(())
}

#[test]
fn get_artifact() -> Result<()> {
    let repo = get_mock_repository();
    let id = DependencyId("artifact1".to_owned());
    let p = repo.get_package(&id, &Version::new(0, 1, 0))?;

    assert!(p.is_some());
    let unwrapped_p = p.unwrap();

    assert_eq!(unwrapped_p.id, id);
    assert_eq!(unwrapped_p.version, Version::new(0, 1, 0));
    Ok(())
}

#[test]
fn resolve() -> Result<()> {
    let repo = get_mock_repository();
    let p = repo.get_package(
        &DependencyId("artifact4".to_owned()),
        &Version::new(0, 1, 0),
    )?;

    assert!(p.is_some());
    let unwrapped_p = p.unwrap();

    let resolved = dependency::resolve(&unwrapped_p, &repo, Default::default())?.collect_vec();

    println!(
        "Resolved deps: {:?}",
        resolved.iter().map(|s| s.0.id.clone()).collect_vec()
    );
    assert_eq!(resolved.len(), 3);

    Ok(())
}

#[test]
fn resolve_locked() -> Result<()> {
    let repo = get_mock_repository();
    let id = DependencyId("artifact4".to_owned());
    let p = repo.get_package(&id, &Version::new(0, 1, 0))?;

    assert!(p.is_some());
    let unwrapped_p = p.unwrap();

    let shared_package = SharedPackageConfig {
        restored_triplet: Default::default(),
        locked_triplet: Default::default(),
        config: unwrapped_p.clone(),
    };

    let resolved = dependency::locked_resolve(&shared_package, &repo, &Default::default())?.collect_vec();

    println!(
        "Resolved deps: {:?}",
        resolved.iter().map(|s| s.0.id.clone()).collect_vec()
    );
    assert_eq!(resolved.len(), 3);

    Ok(())
}

#[test]
fn resolve_fail() -> Result<()> {
    let repo = get_mock_repository();
    let id = DependencyId("artifact5".to_owned());
    let p = repo.get_package(&id, &Version::new(0, 1, 0))?;

    assert!(p.is_some());
    let unwrapped_p = p.unwrap();

    let resolved = dependency::resolve(&unwrapped_p, &repo, &Default::default());

    assert!(resolved.is_err());

    Ok(())
}
