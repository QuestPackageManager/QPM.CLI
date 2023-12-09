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
    repository::multi::MultiDependencyRepository,
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
        let mut repo = MultiDependencyRepository::useful_default_new(self.offline)?;

        let unlocked = self.update || !SharedPackageConfig::exists(".");
        let locked = !unlocked;

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

        let temp_shared_package = locked.then(|| SharedPackageConfig::read(".")).transpose()?;

        let (shared_package, resolved_deps) = match temp_shared_package {
            // only do this if shared and local are the same
            Some(mut temp_shared_package) if package == temp_shared_package.config => {
                // if the same, restore as usual
                println!("Using lock file for restoring");

                temp_shared_package.config = package;
                let dependencies =
                    dependency::locked_resolve(&temp_shared_package, &repo)?.collect_vec();

                (temp_shared_package, dependencies)
            }
            // Unlocked resolve
            _ => {
                println!("Resolving packages");

                let (spc_result, restored_deps) =
                    SharedPackageConfig::resolve_from_package(package, &repo)?;
                (spc_result, restored_deps)
            }
        };

        // write the ndk path to a file if available
        let _config = get_combine_config();

        dependency::restore(".", &shared_package, &resolved_deps, &mut repo)?;

        shared_package.write(".")?;

        Ok(())
    }
}
