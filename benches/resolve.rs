use itertools::Itertools;
use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};
use semver::Version;

use qpm_cli::{
    models::package::{PackageConfigExtensions, SharedPackageConfigExtensions},
    repository::{
        local::FileRepository,
        multi::MultiDependencyRepository,
        qpackages::{self, QPMRepository},
        Repository,
    },
    resolver::dependency,
    tests::mocks::repo::get_mock_repository,
};

use criterion::{Criterion, criterion_group, criterion_main, black_box};

fn resolve(c: &mut Criterion) {
    let repo = get_mock_repository();

    let p = repo
        .get_package("artifact4", &Version::new(0, 1, 0))
        .unwrap();

    assert!(p.is_some());
    let unwrapped_p = p.unwrap();

    c.bench_function("resolve", |b| {
        b.iter(|| {
            dependency::resolve(black_box(&unwrapped_p.config), black_box(&repo))
                .unwrap()
                .collect_vec()
        })
    });
}

// realistic resolve
// chroma
fn real_resolve(c: &mut Criterion) {
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

    let config = p.config;
    // let config = PackageConfig::read("E:\\SSDUse\\ProgrammingProjects\\CLionProjects\\ChromaQuest").unwrap();

    c.bench_function("resolve chroma", |b| {
        b.iter(|| {
            dependency::resolve(black_box(&config), black_box(&repo))
                .unwrap()
                .collect_vec()
        })
    });

    dependency::resolve(&config, &repo).unwrap().collect_vec();
    let (new_config, new_shared_deps) =
        SharedPackageConfig::resolve_from_package(config, &repo).unwrap();
    println!(
        "Resolving for dependencies {:?}",
        new_shared_deps
            .iter()
            .map(|i| &i.config.info.id)
            .collect_vec()
    );
}

criterion_group!(benches, resolve, real_resolve);
criterion_main!(benches);