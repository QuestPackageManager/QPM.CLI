use std::{env, fs::File, io::Read, path::Path};

use clap::Args;

use color_eyre::{
    Section,
    eyre::{Context, Result, bail, eyre},
};
use itertools::Itertools;
use qpm_package::models::{
    package::PackageConfig,
    shared_package::{QPM_SHARED_JSON, SharedPackageConfig},
};
use semver::Version;

use crate::{
    models::package::{PackageConfigExtensions, SharedPackageConfigExtensions},
    repository::{self},
    resolver::dependency,
    terminal::colors::QPMColor,
};

use super::Command;

#[derive(Args, Default)]
pub struct RestoreCommand {
    #[clap(default_value = "false", long, short)]
    update: bool,

    #[clap(long, default_value = "false")]
    offline: bool,
}

#[cfg(feature = "gitoxide")]
pub(crate) fn is_ignored() -> bool {
    gix::open(".").is_ok_and(|r| {
        let Ok(index) = r.index() else { return false };

        let excludes = r.excludes(&index, None, Default::default());

        excludes.is_ok_and(|mut attribute| {
            attribute
                .at_path(QPM_SHARED_JSON, Some(gix::index::entry::Mode::FILE))
                .is_ok_and(|e| e.is_excluded())
        })
    })
}

#[cfg(not(feature = "gitoxide"))]
pub(crate) fn is_ignored() -> bool {
    false
}

impl Command for RestoreCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".").context("Reading package config for restoring")?;
        // optionally does not exist
        let mut shared_package_opt = SharedPackageConfig::exists(".")
            .then(|| SharedPackageConfig::read("."))
            .transpose()?;

        let mut repo = repository::useful_default_new(self.offline)?;

        // only update if:
        // manually
        // no shared.qpm.json
        // dependencies have been updated
        let unlocked = self.update || is_modified(&shared_package_opt, &package);

        if !unlocked && is_ignored() {
            eprintln!("It seems that the current repository has {QPM_SHARED_JSON} ignored. ");
            eprintln!(
                "Please commit it to avoid inconsistent dependency resolving. git add {QPM_SHARED_JSON} --force"
            );
        }

        if unlocked && env::var("CI") == Ok("true".to_string()) {
            eprintln!("Running in CI and using unlocked resolve, this seems like a bug!");
            eprintln!(
                "Make sure {QPM_SHARED_JSON} is not gitignore'd and is comitted in the repository"
            );
        }

        // TODO: sanity check. This used to also require the requested triplet to be
        // present in shared_package.locked_triplet before taking the locked-resolve path;
        // now that there's a single config, `!unlocked` (driven by is_modified) is the only
        // gate. Confirm this still behaves correctly for a shared package restored under
        // the old multi-triplet format or otherwise missing restored_dependencies.
        let resolved_deps = match &mut shared_package_opt {
            // locked resolve
            // only if shared_package is Some() and locked
            Some(shared_package) if !unlocked => {
                // if the same, restore as usual
                println!("Using lock file for restoring");

                // update config
                shared_package.config = package;

                dependency::locked_resolve(shared_package, &repo)?.collect_vec()
            }
            // Unlocked resolve
            _ => {
                println!("Resolving packages");

                let (spc_result, resolved_deps) =
                    SharedPackageConfig::resolve_from_package(package, &repo)?;

                // update shared_package
                shared_package_opt = Some(spc_result);

                resolved_deps
            }
        };

        // write the ndk path to a file if available
        let shared_package = shared_package_opt.expect("SharedPackage is None somehow!");

        // always write to reflect config changes
        dependency::restore(".", &shared_package, &resolved_deps, &mut repo)?;
        shared_package.write(".")?;

        println!(
            "Restored {} with {} dependencies",
            shared_package.config.id.dependency_id_color(),
            resolved_deps.len()
        );

        validate_ndk(&shared_package.config)?;

        Ok(())
    }
}

fn is_modified(shared_package_opt: &Option<SharedPackageConfig>, package: &PackageConfig) -> bool {
    let Some(shared_package) = shared_package_opt else {
        return true;
    };

    // we just naively compare the package configs for now,
    // if they are different, we consider it modified.
    // This means that any change to the package config will cause an unlocked restore, even if the triplet dependencies are not affected.
    // We can optimize this later by only comparing the relevant parts of the package config (like dependencies and triplets).
    if shared_package.config != *package {
        return true;
    }

    // // return true if the triplet is not locked
    // let Some(locked_triplet) = shared_package.locked_triplet.get(triplet_id) else {
    //     return true;
    // };

    // // if the number of dependencies is different, it is modified

    // for (dep_id, dep) in triplet.dependencies.iter() {
    //     // if the dependency is not in the locked triplet, it is modified
    //     let Some(locked_dep) = locked_triplet.restored_dependencies.get(dep_id) else {
    //         return true;
    //     };
    //     if let Some(dep_triplet) = &dep.triplet
    //         && *dep_triplet != locked_dep.restored_triplet
    //     {
    //         return true;
    //     }
    //     if !dep.version_range.matches(&locked_dep.restored_version) {
    //         return true;
    //     }
    // }

    false
}

pub fn validate_ndk(package: &PackageConfig) -> Result<()> {
    let Some(ndk_req) = package.ndk.as_ref() else {
        return Ok(());
    };

    let mut ndk_path_str = String::new();

    // early return, the file doesn't exist nothing to validate
    let ndk_path = Path::new("./ndkpath.txt");
    if ndk_path.exists() {
        let mut ndk_file = File::open(ndk_path)?;

        ndk_file.read_to_string(&mut ndk_path_str)?;
        // validate environment variable if possible
    } else if let Some(ndk_path_env) =
        std::env::var_os("ANDROID_NDK_HOME").or_else(|| std::env::var_os("ANDROID_NDK_LATEST_HOME"))
    {
        ndk_path_str = ndk_path_env.to_str().unwrap().to_string();
    }

    let ndk_path = Path::new(ndk_path_str.trim());
    if ndk_path.as_os_str().is_empty() {
        eprintln!("NDK Path is empty, skipping validate NDK version!");
        return Ok(());
    }

    if !ndk_path.exists() {
        bail!("NDK Path {} does not exist!", ndk_path.display());
    }

    let ndk_version_str = ndk_path.file_name().unwrap().to_str().unwrap();
    let Ok(ndk_version) = Version::parse(ndk_version_str) else {
        println!("Unable to validate {ndk_version_str} is a valid NDK version, skipping");
        return Ok(());
    };

    if !ndk_req.matches(&ndk_version) {
        return Err(
            eyre!("NDK Version {ndk_version} does not satisfy {ndk_req}")
                .suggestion("qpm2 ndk resolve".to_string()),
        );
    }

    Ok(())
}
