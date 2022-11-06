use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
    error::Error,
    path::Path,
};

use color_eyre::{
    eyre::{anyhow, bail, Context, ContextCompat},
    Result,
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use qpm_package::models::{
    backend::PackageVersion,
    dependency::SharedPackageConfig,
    package::{PackageConfig, PackageDependency},
};

use stopwatch::Stopwatch;

use crate::{
    repository::{local::FileRepository, Repository},
    terminal::colors::QPMColor,
    utils::cmake::{write_define_cmake, write_extern_cmake},
};

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
            |id| {
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
                    .sorted()
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

#[cfg(not(feature = "pubgrub_resolve"))]
pub fn resolve<'a>(
    root: &'a PackageConfig,
    repository: &'a impl Repository,
) -> Result<impl Iterator<Ietm = SharedPackageConfig> + 'a> {
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
            bail!("{}", err)
        }
    };
    println!("Took {}ms to dependency resolve", sw.elapsed_ms());
    result
}

#[cfg(feature = "pubgrub_resolve")]
pub fn resolve<'a>(
    root: &'a PackageConfig,
    repository: &'a impl Repository,
) -> Result<impl Iterator<Item = SharedPackageConfig> + 'a> {
    fast_resolve(root, repository)
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
        repository.download_to_cache(&dep.config)?;
        repository.add_to_db_cache(dep.clone(), true)?;
    }

    repository.write_repo()?;

    println!("Copying now");
    FileRepository::copy_from_cache(&shared_package.config, resolved_deps, workspace.as_ref())?;

    write_extern_cmake(shared_package, repository)?;
    write_define_cmake(shared_package)?;
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
                .unwrap()
                .unwrap()
        })
        .dedup_by(|x, y| x.config.info.id == y.config.info.id))
}

pub fn fast_resolve<'a>(
    root: &'a PackageConfig,
    repository: &'a impl Repository,
) -> Result<impl Iterator<Item = SharedPackageConfig> + 'a> {
    let handle_ranges = |dep: &PackageDependency,

                         requirements: &mut HashMap<String, Range<VersionWrapper>>|
     -> Result<()> {
        match requirements.entry(dep.id.clone()) {
            Entry::Occupied(mut o) => {
                let range = req_to_range(dep.version_range.clone());

                if o.get().intersection(&range) != Range::none() {
                    o.insert(o.get().intersection(&range));
                }
            }
            Entry::Vacant(e) => {
                e.insert(req_to_range(dep.version_range.clone()));
            }
        };

        Ok(())
    };
    let handle_deps = |package: &PackageConfig,
                       requirements: &mut HashMap<String, Range<VersionWrapper>>,
                       packages_queue: &mut Vec<PackageConfig>|
     -> Result<()> {
        for dep in &package.dependencies {
            let range = requirements
                .get(&dep.id)
                .with_context(|| format!("No range found for {}?", &dep.id))?;
            let versions = repository
                .get_package_versions(&dep.id)?
                .context("No versions found")?;
            let suitable_version = versions
                .iter()
                .find(|x| range.contains(&x.version.clone().into()));

            if suitable_version.is_none() {
                continue;
            }

            let dep_package = repository
                .get_package(&dep.id, &suitable_version.unwrap().version)?
                .ok_or_else(|| {
                    anyhow!(
                        "Package {}:{} not found",
                        dep.id,
                        suitable_version.unwrap().version
                    )
                })?;

            if dep.additional_data.is_private.unwrap_or(false) {
                continue;
            }
            packages_queue.push(dep_package.config);
        }
        Ok(())
    };

    let sw = Stopwatch::start_new();
    let mut requirements: HashMap<String, Range<VersionWrapper>> = HashMap::new();
    let mut packages_queue: Vec<PackageConfig> = Vec::with_capacity(root.dependencies.len());

    packages_queue.push(root.clone());

    // // root package
    // for dep in &root.dependencies {
    //     handle_ranges(dep, &mut requirements)?;
    // }

    // handle_deps(root, &mut requirements, &mut packages_queue)?;

    loop {
        let package = packages_queue.pop();
        match package {
            Some(o) => {
                for dep in &o.dependencies {
                    handle_ranges(dep, &mut requirements)?;
                }

                handle_deps(&o, &mut requirements, &mut packages_queue)?;
            }
            None => break,
        }
    }

    // finally get all remaining packages
    let results = requirements.into_iter().map(|(id, range)| {
        let versions = repository
            .get_package_versions(&id)
            .expect("Unable to get versions")
            .unwrap();

        let version = versions
            .into_iter()
            .sorted_by(|a, b| a.version.cmp(&b.version))
            .rev() // highest version possible
            .find(|p| range.contains(&p.version.clone().into()))
            .unwrap_or_else(|| {
                panic!(
                    "Unable to find suitable version for id {} range {}",
                    id, range
                )
            });

        repository
            .get_package(&id, &version.version)
            .unwrap()
            .unwrap()
    });
    println!("Took {}ms to dependency resolve", sw.elapsed_ms());

    Ok(results)
}
