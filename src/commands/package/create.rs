use std::path::Path;

use clap::Args;
use owo_colors::OwoColorize;
use qpm_package::models::package::{self, DependencyId, PackageConfig};
use semver::Version;

use crate::{commands::Command, models::package::PackageConfigExtensions};

#[derive(Args, Debug, Clone)]

pub struct PackageOperationCreateArgs {
    /// The name of the package
    pub name: String,
    /// The version of the package
    pub version: Version,
    /// Specify an id, else lowercase will be used
    #[clap(long = "id")]
    pub id: Option<String>,
}

impl Command for PackageOperationCreateArgs {
    fn execute(self) -> color_eyre::Result<()> {
        if PackageConfig::exists(".") {
            println!(
                "{}",
                "Package already existed, not creating a new package!".bright_red()
            );
            println!(
                "Did you try to make a package in the same directory as another, or did you not use a clean folder?"
            );
            return Ok(());
        }

        // id is optional so we need to check if it's defined, else use the name to lowercase
        let id = match self.id {
            Option::Some(s) => s,
            Option::None => self.name.to_lowercase(),
        };

        let package = PackageConfig {
            id: DependencyId(id),
            version: self.version,
            additional_data: Default::default(),
            triplets: Default::default(),
            cmake: Default::default(),
            toolchain_out: Some(Path::new("toolchain.json").to_owned()),

            shared_directory: Path::new("shared").to_owned(),
            dependencies_directory: Path::new("extern").to_owned(),
            workspace: Default::default(),
            config_version: package::package_target_version(),
        };

        package.write(".")?;
        Ok(())
    }
}
