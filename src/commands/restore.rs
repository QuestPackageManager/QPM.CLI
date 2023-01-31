use std::{env, fs, path::Path};

use clap::Args;

use git2::Status;
use itertools::Itertools;
use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};

use crate::{
    models::{
        config::get_combine_config,
        package::{PackageConfigExtensions, SharedPackageConfigExtensions, SharedPackageFileName},
    },
    repository::multi::MultiDependencyRepository,
    resolver::dependency,
};

use super::Command;

#[derive(Args)]
pub struct RestoreCommand {
    #[clap(default_value = "false", long, short)]
    update: bool,
}

fn is_ignored() -> bool {
    git2::Repository::open(".").is_ok_and(|r| {
        r.is_path_ignored(SharedPackageFileName).contains(&true)
            || r.status_file(Path::new(SharedPackageFileName))
                .is_ok_and(|s| s.is_empty())
    })
}

impl Command for RestoreCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        let shared_package: SharedPackageConfig;
        let mut repo = MultiDependencyRepository::useful_default_new()?;

        let unlocked = self.update || !SharedPackageConfig::check(".");

        if !unlocked && is_ignored() {
            eprintln!("It seems that the current repository has {SharedPackageFileName} ignored. ");
            eprintln!("Please commit it to avoid inconsistent dependency resolving. git add {SharedPackageFileName} --force");
        }

        if unlocked && env::var("CI").contains(&"true".to_string()) {
            eprintln!("Running in CI and using unlocked resolve, this seems like a bug!");
            eprintln!("Make sure {SharedPackageFileName} is not gitignore'd and is comitted in the repository");
        }

        let resolved_deps = match unlocked {
            false => {
                println!("Using lock file for restoring");

                let mut temp_shared_package = SharedPackageConfig::read(".")?;
                temp_shared_package.config = package;
                shared_package = temp_shared_package;

                dependency::locked_resolve(&shared_package, &repo)?.collect_vec()
            }
            true => {
                println!("Resolving packages");

                let (s, d) = SharedPackageConfig::resolve_from_package(package, &repo)?;
                shared_package = s;
                d
            }
        };

        // create used dirs
        fs::create_dir_all("src")?;
        fs::create_dir_all("include")?;
        fs::create_dir_all(&shared_package.config.shared_dir)?;

        // write the ndk path to a file if available
        let _config = get_combine_config();

        dependency::restore(".", &shared_package, &resolved_deps, &mut repo)?;

        shared_package.write(".")?;

        Ok(())
    }
}
