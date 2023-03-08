use clap::Args;
use owo_colors::OwoColorize;
use qpm_package::models::package::PackageConfig;

use crate::{
    models::package::PackageConfigExtensions, repository::multi::MultiDependencyRepository,
    resolver::dependency::resolve,
};

use super::Command;

#[derive(Args)]
pub struct CollapseCommand {
    #[clap(long)]
    offline: bool
}

impl Command for CollapseCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        let binding = MultiDependencyRepository::useful_default_new(self.offline)?;
        let resolved = resolve(&package, &binding)?;
        for shared_package in resolved {
            println!(
                "{} --> {} ({} restored dependencies)",
                &shared_package.config.info.id.bright_red(),
                &shared_package.config.info.version.bright_green(),
                shared_package
                    .restored_dependencies
                    .len()
                    .to_string()
                    .yellow()
            );

            for shared_dep in shared_package.restored_dependencies.iter() {
                println!(
                    " - {}: ({}) --> {}",
                    &shared_dep.dependency.id,
                    &shared_dep.dependency.version_range,
                    &shared_dep.version
                );
            }
        }
        Ok(())
    }
}
