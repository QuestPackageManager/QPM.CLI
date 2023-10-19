use color_eyre::Result;
use itertools::Itertools;
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
    let p = repo.get_package("artifact1", &Version::new(0, 1, 0))?;

    assert!(p.is_some());
    let unwrapped_p = p.unwrap();

    assert_eq!(unwrapped_p.config.info.id, "artifact1");
    assert_eq!(unwrapped_p.config.info.version, Version::new(0, 1, 0));
    Ok(())
}

#[test]
fn resolve() -> Result<()> {
    let repo = get_mock_repository();
    let p = repo.get_package("artifact4", &Version::new(0, 1, 0))?;

    assert!(p.is_some());
    let unwrapped_p = p.unwrap();

    let resolved = dependency::resolve(&unwrapped_p.config, &repo)?.collect_vec();

    println!(
        "Resolved deps: {:?}",
        resolved.iter().map(|s| s.config.info.id.clone())
    );
    assert_eq!(resolved.len(), 3);

    Ok(())
}

#[test]
fn resolve_locked() -> Result<()> {
    let repo = get_mock_repository();
    let p = repo.get_package("artifact4", &Version::new(0, 1, 0))?;

    assert!(p.is_some());
    let unwrapped_p = p.unwrap();

    let resolved = dependency::locked_resolve(&unwrapped_p, &repo)?.collect_vec();

    println!(
        "Resolved deps: {:?}",
        resolved.iter().map(|s| s.config.info.id.clone())
    );
    assert_eq!(resolved.len(), 3);

    Ok(())
}

#[test]
fn resolve_fail() -> Result<()> {
    let repo = get_mock_repository();
    let p = repo.get_package("artifact5", &Version::new(0, 1, 0))?;

    assert!(p.is_some());
    let unwrapped_p = p.unwrap();

    let resolved = dependency::resolve(&unwrapped_p.config, &repo);

    assert!(resolved.is_err());

    Ok(())
}
