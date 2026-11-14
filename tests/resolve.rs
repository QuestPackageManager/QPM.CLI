mod mocks;
#[path = "support/mod.rs"]
mod support;

use color_eyre::Result;
use itertools::Itertools;
use qpm_cli::{repository::Repository, services::restore::PackageRestorer};
use qpm_package::models::package::DependencyId;
use semver::Version;

use mocks::repo::get_mock_repository;

#[test]
fn get_artifact_names() -> Result<()> {
    let repo = get_mock_repository();
    let names = repo.get_package_names()?;

    let packages_name_mapped = repo.artifacts().keys().cloned().collect_vec();

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

    assert_eq!(unwrapped_p.config.id, id);
    assert_eq!(unwrapped_p.config.version, Version::new(0, 1, 0));
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

    let restorer = PackageRestorer::resolve(unwrapped_p.config, &repo)?;
    let resolved = restorer.resolved_deps();

    println!(
        "Resolved deps: {:?}",
        resolved.iter().map(|s| s.config.id.clone()).collect_vec()
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

    let restorer = PackageRestorer::resolve(unwrapped_p.config, &repo)?;
    let shared_package = restorer.shared_package().clone();

    let restorer = PackageRestorer::locked_resolve(&shared_package, &repo)?;
    let resolved = restorer.resolved_deps();

    println!(
        "Resolved deps: {:?}",
        resolved.iter().map(|s| s.config.id.clone()).collect_vec()
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

    let resolved = PackageRestorer::resolve(unwrapped_p.config, &repo);

    assert!(resolved.is_err());

    Ok(())
}

// Graphs built inline on an in-memory repository

mod graphs {
    use std::collections::HashMap;

    use qpm_cli::services::restore::PackageRestorer;
    use qpm_package::models::{
        package::{DependencyId, PackageConfig, PackageDependency},
        shared_package::SharedPackageConfig,
    };
    use semver::{Version, VersionReq};

    use crate::{mocks::mock_repository::MockRepository, support::locked};

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn pkg(name: &str, version: &str, deps: &[(&str, &str)]) -> PackageConfig {
        PackageConfig {
            id: DependencyId(name.to_string()),
            version: v(version),
            dependencies: deps
                .iter()
                .map(|(id, range)| {
                    (
                        DependencyId(id.to_string()),
                        PackageDependency {
                            version_range: VersionReq::parse(range).unwrap(),
                            ..Default::default()
                        },
                    )
                })
                .collect(),
            ..Default::default()
        }
    }

    fn repo(packages: &[PackageConfig]) -> MockRepository {
        let repo = MockRepository::new(false);
        for p in packages {
            repo.insert(p.clone());
        }
        repo
    }

    /// id → resolved version
    fn resolved(root: PackageConfig, repo: &MockRepository) -> HashMap<String, Version> {
        PackageRestorer::resolve(root, repo)
            .expect("resolution should succeed")
            .resolved_deps()
            .iter()
            .map(|d| (d.config.id.0.clone(), d.config.version.clone()))
            .collect()
    }

    fn resolve_err(root: PackageConfig, repo: &MockRepository) -> String {
        let err = PackageRestorer::resolve(root, repo)
            .err()
            .expect("resolution should fail");
        format!("{err:?}")
    }

    #[test]
    fn picks_highest_version_in_range() {
        let repo = repo(&[
            pkg("a", "1.0.0", &[]),
            pkg("a", "1.2.0", &[]),
            pkg("a", "2.0.0", &[]),
        ]);
        let root = pkg("root", "1.0.0", &[("a", "^1")]);

        assert_eq!(resolved(root, &repo), HashMap::from([("a".into(), v("1.2.0"))]));
    }

    #[test]
    fn diamond_picks_one_version_satisfying_both() {
        let repo = repo(&[
            pkg("a", "1.0.0", &[("c", "^1.0")]),
            pkg("b", "1.0.0", &[("c", ">=1.1, <1.3")]),
            pkg("c", "1.0.0", &[]),
            pkg("c", "1.1.0", &[]),
            pkg("c", "1.2.0", &[]),
            pkg("c", "1.3.0", &[]),
        ]);
        let root = pkg("root", "1.0.0", &[("a", "*"), ("b", "*")]);

        let resolved = resolved(root, &repo);
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved["c"], v("1.2.0"));
    }

    #[test]
    fn unsatisfiable_diamond_names_the_conflict() {
        let repo = repo(&[
            pkg("a", "1.0.0", &[("conflicted", "^1")]),
            pkg("b", "1.0.0", &[("conflicted", "^2")]),
            pkg("conflicted", "1.0.0", &[]),
            pkg("conflicted", "2.0.0", &[]),
        ]);
        let root = pkg("root", "1.0.0", &[("a", "*"), ("b", "*")]);

        assert!(resolve_err(root, &repo).contains("conflicted"));
    }

    #[test]
    fn missing_transitive_package_fails() {
        let repo = repo(&[pkg("a", "1.0.0", &[("ghost", "^1")])]);
        let root = pkg("root", "1.0.0", &[("a", "*")]);

        assert!(resolve_err(root, &repo).contains("ghost"));
    }

    #[test]
    fn root_dev_dependencies_resolve_but_not_those_of_dependencies() {
        let mut a = pkg("a", "1.0.0", &[]);
        a.dev_dependencies = pkg("_", "0.0.0", &[("a_dev", "*")]).dependencies;
        // a_dev isn't in the repository, so resolving it would fail
        let repo = repo(&[a, pkg("tool", "1.0.0", &[])]);

        let mut root = pkg("root", "1.0.0", &[("a", "*")]);
        root.dev_dependencies = pkg("_", "0.0.0", &[("tool", "*")]).dependencies;

        let resolved = resolved(root, &repo);
        assert_eq!(
            resolved,
            HashMap::from([("a".into(), v("1.0.0")), ("tool".into(), v("1.0.0"))])
        );
    }

    #[test]
    fn locked_resolve_keeps_locked_versions_over_newer_ones() {
        let repo = repo(&[pkg("a", "1.0.0", &[]), pkg("a", "1.5.0", &[])]);
        let shared = SharedPackageConfig {
            config: pkg("root", "1.0.0", &[("a", "^1")]),
            restored_dependencies: HashMap::from([(DependencyId("a".into()), locked(v("1.0.0")))]),
            env: Default::default(),
        };

        let restorer = PackageRestorer::locked_resolve(&shared, &repo).unwrap();
        let versions: Vec<_> = restorer
            .resolved_deps()
            .iter()
            .map(|d| d.config.version.clone())
            .collect();
        assert_eq!(versions, vec![v("1.0.0")]);
    }

    fn assert_cycle(root: PackageConfig, repo: &MockRepository, cycle: &str) {
        let err = resolve_err(root, repo);
        assert!(
            err.contains(&format!("Cyclic dependency detected: {cycle}")),
            "expected cycle {cycle}, got: {err}"
        );
    }

    #[test]
    fn self_dependency_is_rejected() {
        let repo = repo(&[pkg("a", "1.0.0", &[("a", "*")])]);
        assert_cycle(pkg("root", "1.0.0", &[("a", "*")]), &repo, "a -> a");
    }

    #[test]
    fn two_package_cycle_is_rejected() {
        let repo = repo(&[
            pkg("a", "1.0.0", &[("b", "*")]),
            pkg("b", "1.0.0", &[("a", "*")]),
        ]);
        assert_cycle(pkg("root", "1.0.0", &[("a", "*")]), &repo, "a -> b -> a");
    }

    #[test]
    fn cycle_through_root_is_rejected() {
        let repo = repo(&[pkg("a", "1.0.0", &[("root", "*")])]);
        assert_cycle(
            pkg("root", "1.0.0", &[("a", "*")]),
            &repo,
            "root -> a -> root",
        );
    }

    #[test]
    fn longer_cycle_is_rejected() {
        let repo = repo(&[
            pkg("a", "1.0.0", &[("b", "*")]),
            pkg("b", "1.0.0", &[("c", "*")]),
            pkg("c", "1.0.0", &[("a", "*")]),
        ]);
        assert_cycle(
            pkg("root", "1.0.0", &[("a", "*")]),
            &repo,
            "a -> b -> c -> a",
        );
    }

    /// Two paths reaching the same package is a diamond, not a cycle
    #[test]
    fn shared_dependency_is_not_a_cycle() {
        let repo = repo(&[
            pkg("a", "1.0.0", &[("c", "*")]),
            pkg("b", "1.0.0", &[("c", "*")]),
            pkg("c", "1.0.0", &[]),
        ]);
        assert_eq!(
            resolved(pkg("root", "1.0.0", &[("a", "*"), ("b", "*")]), &repo).len(),
            3
        );
    }

    #[test]
    fn locked_resolve_rejects_a_cycle_in_the_lock_file() {
        let repo = repo(&[
            pkg("a", "1.0.0", &[("b", "*")]),
            pkg("b", "1.0.0", &[("a", "*")]),
        ]);
        let shared = SharedPackageConfig {
            config: pkg("root", "1.0.0", &[("a", "*")]),
            restored_dependencies: HashMap::from([
                (DependencyId("a".into()), locked(v("1.0.0"))),
                (DependencyId("b".into()), locked(v("1.0.0"))),
            ]),
            env: Default::default(),
        };

        let err = PackageRestorer::locked_resolve(&shared, &repo)
            .err()
            .expect("a cyclic lock file should fail");
        assert!(format!("{err:?}").contains("Cyclic dependency detected: a -> b -> a"));
    }
}
