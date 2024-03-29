use std::{env, path::Path};

use clap::Args;

use itertools::Itertools;
use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};

use crate::{
    models::{
        config::get_combine_config,
        package::{
            PackageConfigExtensions, SharedPackageConfigExtensions, SHARED_PACKAGE_FILE_NAME,
        },
    },
    repository::{self},
    resolver::dependency,
};

use super::Command;

#[derive(Args)]
pub struct RestoreCommand {
    #[clap(default_value = "false", long, short)]
    update: bool,

    #[clap(long, default_value = "false")]
    offline: bool,
}

fn is_ignored() -> bool {
    git2::Repository::open(".").is_ok_and(|r| {
        r.is_path_ignored(SHARED_PACKAGE_FILE_NAME) == Ok(true)
            || r.status_file(Path::new(SHARED_PACKAGE_FILE_NAME))
                .is_ok_and(|s| s.is_ignored())
    })
}

impl Command for RestoreCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        // optionally does not exist
        let mut shared_package_opt = SharedPackageConfig::exists(".")
            .then(|| SharedPackageConfig::read("."))
            .transpose()?;

        let mut repo = repository::useful_default_new(self.offline)?;

        // only update if:
        // manually
        // no shared.qpm.json
        // dependencies have been updated
        let unlocked = self.update
            || shared_package_opt.is_none()
            || shared_package_opt.as_ref().is_some_and(|shared_package| {
                shared_package.config.dependencies != package.dependencies
            });

        if !unlocked && is_ignored() {
            eprintln!(
                "It seems that the current repository has {SHARED_PACKAGE_FILE_NAME} ignored. "
            );
            eprintln!("Please commit it to avoid inconsistent dependency resolving. git add {SHARED_PACKAGE_FILE_NAME} --force");
        }

        if unlocked && env::var("CI") == Ok("true".to_string()) {
            eprintln!("Running in CI and using unlocked resolve, this seems like a bug!");
            eprintln!("Make sure {SHARED_PACKAGE_FILE_NAME} is not gitignore'd and is comitted in the repository");
        }

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

                let (spc_result, restored_deps) =
                    SharedPackageConfig::resolve_from_package(package, &repo)?;
                // update shared_package
                shared_package_opt = Some(spc_result);

                restored_deps
            }
        };

        // write the ndk path to a file if available
        let _config = get_combine_config();

        let shared_package = shared_package_opt.expect("SharedPackage is None somehow!");

        // always write to reflect config changes
        dependency::restore(".", &shared_package, &resolved_deps, &mut repo)?;
        shared_package.write(".")?;

        Ok(())
    }
}
