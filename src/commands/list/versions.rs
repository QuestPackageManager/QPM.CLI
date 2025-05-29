use clap::Args;
use color_eyre::eyre::ContextCompat;
use owo_colors::OwoColorize;
use qpm_package::models::package::DependencyId;

use crate::{
    commands::Command,
    repository::{self, Repository},
};

#[derive(Args, Debug, Clone)]
pub struct PackageCommand {
    pub package: String,
    #[clap(short, long)]
    pub latest: bool,

    #[clap(long, default_value = "false")]
    offline: bool,
}

impl Command for PackageCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let versions = repository::useful_default_new(self.offline)?
            .get_package_versions(&DependencyId(self.package.clone()))?;

        match versions {
            Some(package_versions) => {
                if self.latest {
                    println!(
                        "The latest version for package {} is {}",
                        self.package.bright_red(),
                        package_versions.first().unwrap().to_string().bright_green()
                    );

                    return Ok(());
                }

                println!(
                    "Package {} has {} versions on qpackages.com:",
                    self.package.bright_red(),
                    package_versions.len().bright_yellow()
                );
                for package_version in package_versions.iter().rev() {
                    println!(" - {}", package_version.to_string().bright_green());
                }
            }
            _ => {
                println!(
                    "Package {} either did not exist or has no versions on qpackages.com",
                    self.package.bright_red()
                );
            }
        }
        Ok(())
    }
}
