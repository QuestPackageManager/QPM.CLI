use itertools::Itertools;
use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};
use semver::Version;

use crate::{
    models::package::{SharedPackageConfigExtensions, PackageConfigExtensions},
    repository::{
        local::FileRepository,
        multi::MultiDependencyRepository,
        qpackages::{self, QPMRepository},
        Repository,
    },
    resolver::dependency,
    tests::mocks::repo::get_mock_repository,
};

extern crate test;

#[bench]
fn resolve(b: &mut test::Bencher) {
    let repo = get_mock_repository();

    let p = repo
        .get_package("artifact4", &Version::new(0, 1, 0))
        .unwrap();

    assert!(p.is_some());
    let unwrapped_p = p.unwrap();

    b.iter(|| {
        let _resolved = dependency::resolve(&unwrapped_p.config, &repo)
            .unwrap()
            .collect_vec();
    })
}

// realistic resolve
// chroma
#[bench]
fn real_resolve(b: &mut test::Bencher) {
    let repo = QPMRepository::default();

    let latest_version = repo
        .get_package_versions("chroma")
        .unwrap()
        .unwrap()
        .into_iter()
        .sorted_by(|a, b| a.version.cmp(&b.version))
        .last()
        .unwrap();

    let p = repo
        .get_package("chroma", &latest_version.version)
        .unwrap()
        .unwrap();

        // let config = p.config;
        let config = PackageConfig::read("E:\\SSDUse\\ProgrammingProjects\\CLionProjects\\ChromaQuest").unwrap();

    b.iter(|| {
        dependency::resolve(&config, &repo).unwrap().collect_vec()
        // println!("Resolved {:?}", resolved.iter().map(|i| format!("{}:{}", i.config.info.id, i.config.info.version)).collect_vec());
    });

    
    dependency::resolve(&config, &repo).unwrap().collect_vec();
    let (new_config, new_shared_deps) = SharedPackageConfig::resolve_from_package(config, &repo).unwrap();
    println!(
        "Resolving for dependencies {:?}",
        new_shared_deps.iter().map(|i| &i.config.info.id).collect_vec()
    );
}
