use std::{borrow::Borrow, error::Error, path::Path, vec::IntoIter};

use crate::{
    models::package::SharedPackageConfigExtensions,
    repository::{local::FileRepository, Repository},
    terminal::colors::QPMColor,
    utils::cmake::write_cmake,
};
use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use itertools::Itertools;
use qpm_package::models::{
    backend::PackageVersion, dependency::SharedPackageConfig, package::PackageConfig,
};
use stopwatch::Stopwatch;

use pubgrub::{
    error::PubGrubError,
    range::Range,
    report::{DefaultStringReporter, Reporter},
    solver::{Dependencies, DependencyProvider},
};

use super::semver::{req_to_range, VersionWrapper};
pub struct PackageDependencyResolver<'a, 'b, R>
where
    R: Repository,
{
    root: &'a PackageConfig,
    repo: &'b R,
}

impl<'a, 'b, R: Repository> DependencyProvider<String, VersionWrapper>
    for PackageDependencyResolver<'a, 'b, R>
{
    fn choose_package_version<T: Borrow<String>, U: Borrow<Range<VersionWrapper>>>(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> Result<(T, Option<VersionWrapper>), Box<dyn Error>> {
        let package = pubgrub::solver::choose_package_with_fewest_versions(
            |id| -> IntoIter<VersionWrapper> {
                if id == &self.root.info.id {
                    let v: VersionWrapper = self.root.info.version.clone().into();
                    return vec![v].into_iter();
                }

                self.repo
                    .get_package_versions(id)
                    .unwrap_or_else(|_| panic!("Unable to make request"))
                    .unwrap_or_else(|| panic!("Unable to find versions for package {id}"))
                    .into_iter()
                    .map(|pv: PackageVersion| pv.version.into())
                    .collect_vec()
                    .into_iter()
            },
            potential_packages,
        );

        Ok(package)
    }

    fn get_dependencies(
        &self,
        id: &String,
        version: &VersionWrapper,
    ) -> Result<Dependencies<String, VersionWrapper>, Box<dyn Error>> {
        // Root dependencies
        if id == &self.root.info.id && version == &self.root.info.version {
            // resolve dependencies of root
            let deps = self
                .root
                .dependencies
                .iter()
                .map(|dep| {
                    let id = &dep.id;
                    let version = req_to_range(dep.version_range.clone());
                    (id.clone(), version)
                })
                .collect();
            return Ok(Dependencies::Known(deps));
        }

        // Find dependencies of depenedencies
        let package = self
            .repo
            .get_package(id, &version.clone().into())
            .with_context(|| format!("Could not find package {id} with version {version}"))?
            .unwrap();

        let deps = package
            .config
            .dependencies
            .into_iter()
            // remove any private dependencies
            .filter(|dep| !dep.additional_data.is_private.unwrap_or(false))
            .map(|dep| {
                let id = dep.id;
                let version = req_to_range(dep.version_range);
                (id, version)
            })
            .collect();
        Ok(Dependencies::Known(deps))
    }
}

pub fn resolve<'a>(
    root: &'a PackageConfig,
    repository: &'a impl Repository,
) -> Result<impl Iterator<Item = SharedPackageConfig> + 'a> {
    let resolver = PackageDependencyResolver {
        root,
        repo: repository,
    };
    let sw = Stopwatch::start_new();
    let result = match pubgrub::solver::resolve(
        &resolver,
        root.info.id.clone(),
        root.info.version.clone(),
    ) {
        Ok(deps) => Ok(deps.into_iter().filter_map(move |(id, version)| {
            if id == root.info.id && version == root.info.version {
                return None;
            }

            repository.get_package(&id, &version.into()).unwrap()
        })),

        Err(PubGrubError::NoSolution(tree)) => {
            let report = DefaultStringReporter::report(&tree);
            bail!("failed to resolve dependencies: \n{}", report)
        }
        Err(err) => {
            bail!("pubgrub: {err}\n{err:?}");
        }
    };
    println!("Took {}ms to dependency resolve", sw.elapsed_ms());
    result
}

pub fn restore<P: AsRef<Path>>(
    workspace: P,
    shared_package: &SharedPackageConfig,
    resolved_deps: &[SharedPackageConfig],
    repository: &mut impl Repository,
) -> Result<()> {
    for dep in resolved_deps {
        println!(
            "Pulling {}:{}",
            &dep.config.info.id.dependency_id_color(),
            &dep.config
                .info
                .version
                .to_string()
                .dependency_version_color()
        );
        repository.download_to_cache(&dep.config).with_context(|| {
            format!(
                "Requesting {}:{}",
                dep.config.info.id.dependency_id_color(),
                dep.config.info.version.version_id_color()
            )
        })?;
        repository.add_to_db_cache(dep.clone(), true)?;
    }

    repository.write_repo()?;

    println!("Copying now");
    FileRepository::copy_from_cache(&shared_package.config, resolved_deps, workspace.as_ref())?;

    write_cmake(shared_package, repository)?;
    shared_package.try_write_toolchain(repository)?;

    Ok(())
}

pub fn locked_resolve<'a, R: Repository>(
    root: &'a SharedPackageConfig,
    repository: &'a R,
) -> Result<impl Iterator<Item = SharedPackageConfig> + 'a> {
    // TODO: ensure restored dependencies take precedence over
    Ok(root
        .restored_dependencies
        .iter()
        .map(|d| {
            repository
                .get_package(&d.dependency.id, &d.version)
                .unwrap_or_else(|e| {
                    panic!(
                        "Encountered an issue resolving for package {}:{}, {e}",
                        d.dependency.id, d.version
                    )
                })
                .unwrap_or_else(|| panic!("No package found for {}:{}", d.dependency.id, d.version))
        })
        .dedup_by(|x, y| x.config.info.id == y.config.info.id))
}
