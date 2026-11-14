#![cfg(feature = "network_test")]

//! Tests against the live qpackages backend. They pick packages from whatever the backend
//! currently lists instead of hard-coding ids/versions, so they keep working as its contents
//! change.

use std::collections::HashMap;

use color_eyre::{Result, eyre::OptionExt};
use qpm_package::models::package::{DependencyId, PackageConfig, PackageDependency};
use semver::{Version, VersionReq};
use tempfile::tempdir;

use qpm_cli::{
    models::package_files::PackageIdPath,
    repository::{self, Artifact, Repository, file::FileRepository, qpackages::QPMRepository},
    services::{
        network::{DEFAULT_TIMEOUT_MS, build_agent},
        restore::PackageRestorer,
    },
};

/// The qpackages index, for read-only queries
fn qpackages() -> QPMRepository {
    QPMRepository::new(build_agent(DEFAULT_TIMEOUT_MS))
}

/// Some package the backend serves, at its newest version
fn some_published_package(repo: &QPMRepository) -> Result<Artifact> {
    let id = repo
        .get_package_names()?
        .into_iter()
        .next()
        .ok_or_eyre("backend lists no packages")?;
    let version = repo
        .get_package_versions(&id)?
        .ok_or_eyre("listed package has no versions")?
        .first()
        .cloned()
        .ok_or_eyre("listed package has an empty version list")?;
    repo.get_package(&id, &version)?
        .ok_or_eyre("listed package version is not retrievable")
}

/// A root package depending on `id` with `range`
fn root_depending_on(id: &DependencyId, range: &str) -> PackageConfig {
    PackageConfig {
        id: DependencyId("root".to_string()),
        version: Version::new(0, 0, 0),
        shared_directory: "shared".into(),
        dependencies_directory: "extern".into(),
        dependencies: HashMap::from([(
            id.clone(),
            PackageDependency {
                version_range: VersionReq::parse(range).unwrap(),
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

#[test]
fn lists_packages() -> Result<()> {
    assert!(!qpackages().get_package_names()?.is_empty());
    Ok(())
}

#[test]
fn package_versions_are_sorted_highest_first() -> Result<()> {
    let repo = qpackages();
    let id = repo.get_package_names()?.into_iter().next().unwrap();

    let versions = repo
        .get_package_versions(&id)?
        .ok_or_eyre("listed package has no versions")?;

    assert!(!versions.is_empty());
    assert!(versions.windows(2).all(|pair| pair[0] > pair[1]));
    Ok(())
}

#[test]
fn unknown_package_has_no_versions() -> Result<()> {
    let versions =
        qpackages().get_package_versions(&DependencyId("qpm-test-no-such-package".into()))?;
    assert!(versions.is_none_or(|v| v.is_empty()));
    Ok(())
}

#[test]
fn get_package_returns_config_and_download_url() -> Result<()> {
    let repo = qpackages();
    let artifact = some_published_package(&repo)?;

    let refetched = repo
        .get_package(&artifact.config.id, &artifact.config.version)?
        .expect("package should be retrievable by id and version");
    assert_eq!(refetched.config.id, artifact.config.id);
    assert_eq!(refetched.config.version, artifact.config.version);
    assert!(
        refetched.qpkg_url.is_some(),
        "qpackages artifacts must say where to download them from"
    );
    Ok(())
}

#[test]
fn resolve_picks_highest_matching_version() -> Result<()> {
    let repo = qpackages();
    let artifact = some_published_package(&repo)?;

    let restorer = PackageRestorer::resolve(root_depending_on(&artifact.config.id, "*"), &repo)?;

    let resolved = restorer
        .resolved_deps()
        .iter()
        .find(|dep| dep.config.id == artifact.config.id)
        .expect("the dependency should be resolved");
    assert_eq!(resolved.config.version, artifact.config.version);
    Ok(())
}

#[test]
fn resolve_fails_when_no_version_matches() -> Result<()> {
    let repo = qpackages();
    let artifact = some_published_package(&repo)?;

    let result =
        PackageRestorer::resolve(root_depending_on(&artifact.config.id, ">=99999.0.0"), &repo);

    let err = result
        .err()
        .expect("an unsatisfiable range must not resolve");
    assert!(
        format!("{err:?}").contains("failed to resolve dependencies"),
        "unexpected error: {err:?}"
    );
    Ok(())
}

/// Restoring downloads into the cache; if the cached files are later deleted (but the registry
/// still lists the package), restoring again downloads them again instead of failing.
#[test]
fn restore_redownloads_deleted_cache_files() -> Result<()> {
    let agent = build_agent(DEFAULT_TIMEOUT_MS);
    let cache = tempdir()?;
    let workspace = tempdir()?;

    let artifact = some_published_package(&qpackages())?;
    let root = root_depending_on(
        &artifact.config.id,
        &format!("={}", artifact.config.version),
    );
    let src_path = PackageIdPath::new(artifact.config.id.clone())
        .version(artifact.config.version.clone())
        .src_path(cache.path());

    let restore = || -> Result<()> {
        let mut repo = repository::default_repository_stack(cache.path().into(), Some(&agent))?;
        let restorer = PackageRestorer::resolve(root.clone(), &repo)?;
        let file_repo = FileRepository::read(cache.path().into())?;
        restorer.restore(Some(&agent), workspace.path(), &mut repo, &file_repo, false)
    };

    restore()?;
    assert!(
        src_path.exists(),
        "first restore should download into the cache"
    );

    std::fs::remove_dir_all(&src_path)?;
    restore()?;
    assert!(src_path.exists(), "second restore should download it again");

    Ok(())
}
