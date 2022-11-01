use std::{borrow::Borrow, error::Error};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use itertools::Itertools;
use qpm_package::models::{
    backend::PackageVersion, dependency::SharedPackageConfig, package::PackageConfig,
};

use crate::repository::Repository;

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
        Ok(pubgrub::solver::choose_package_with_fewest_versions(
            |id| {
                if id == &self.root.info.id {
                    panic!("Resolving {id} which is also root package id!");
                }
                // if id == &self.root.info.id {
                //     // TODO: Make this not happe
                //     let v: VersionWrapper = self.root.info.version.into();
                //     return vec![v].into_iter();
                // }

                self.repo
                    .get_package_versions(id)
                    .unwrap_or_else(|_| panic!("Unable to make request"))
                    .unwrap_or_else(|| panic!("Unable to find versions for package {id}"))
                    .into_iter()
                    .map(|pv: PackageVersion| pv.version.into())
            },
            potential_packages,
        ))
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

pub fn resolve<'a, R: Repository>(
    root: &'a PackageConfig,
    repository: &'a R,
) -> Result<impl Iterator<Item = SharedPackageConfig> + 'a> {
    let resolver = PackageDependencyResolver {
        root,
        repo: repository,
    };
    match pubgrub::solver::resolve(&resolver, root.info.id.clone(), root.info.version.clone()) {
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
            bail!("{}", err)
        }
    }
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
                .get_package(&d.dependency.id, &d.version).unwrap().unwrap()
        })
        .dedup_by(|x, y| x.config.info.id == y.config.info.id))
}