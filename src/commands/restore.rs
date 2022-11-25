use std::{fs, io::Write};

use clap::Args;
use color_eyre::eyre::Context;
use itertools::Itertools;
use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};

use crate::{
    models::{
        config::get_combine_config,
        package::{PackageConfigExtensions, SharedPackageConfigExtensions},
    },
    repository::multi::MultiDependencyRepository,
    resolver::dependency,
};

use super::Command;

#[derive(Args)]
pub struct RestoreCommand {
    #[clap(default_value = "false", long, short)]
    locked: bool,
}

impl Command for RestoreCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        let shared_package: SharedPackageConfig;
        let mut repo = MultiDependencyRepository::useful_default_new()?;

        let resolved_deps = match self.locked {
            true => {
                shared_package = SharedPackageConfig::read(".")?;
                dependency::locked_resolve(&shared_package, &repo)?.collect_vec()
            }
            false => {
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
        let config = get_combine_config();

        dependency::restore(".", &shared_package, &resolved_deps, &mut repo)?;

        if !self.locked {
            shared_package.write(".")?;
        }

        Ok(())
    }
}
