use std::path::Path;

use clap::Args;
use owo_colors::OwoColorize;
use qpm_package::models::{
    extra::AdditionalPackageMetadata,
    package::{PackageConfig, PackageMetadata},
};
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
    /// Branch name of a Github repo. Only used when a valid github url is provided
    #[clap(long = "branchName")]
    pub branch_name: Option<String>,
    /// Specify that this package is headers only and does not contain a .so or .a file
    #[clap(long = "headersOnly")]
    pub headers_only: Option<bool>,
    /// Specify that this package is static linking
    #[clap(long = "staticLinking")]
    pub static_linking: Option<bool>,
    /// Specify the download link for a release .so or .a file
    #[clap(long = "soLink")]
    pub so_link: Option<String>,
    /// Specify the download link for a debug .so or .a files (usually from the obj folder)
    #[clap(long = "debugSoLink")]
    pub debug_so_link: Option<String>,
    /// Override the downloaded .so or .a filename with this name instead.
    #[clap(long = "overrideSoName")]
    pub override_so_name: Option<String>,
}

impl Command for PackageOperationCreateArgs {
    fn execute(self) -> color_eyre::Result<()> {
        if PackageConfig::exists(".") {
            println!(
                "{}",
                "Package already existed, not creating a new package!".bright_red()
            );
            println!("Did you try to make a package in the same directory as another, or did you not use a clean folder?");
            return Ok(());
        }

        let additional_data = AdditionalPackageMetadata {
            branch_name: self.branch_name,
            headers_only: self.headers_only,
            static_linking: self.static_linking,
            so_link: self.so_link,
            debug_so_link: self.debug_so_link,
            override_so_name: self.override_so_name,
            ..Default::default()
        };

        // id is optional so we need to check if it's defined, else use the name to lowercase
        let id = match self.id {
            Option::Some(s) => s,
            Option::None => self.name.to_lowercase(),
        };

        let package_info = PackageMetadata {
            id,
            name: self.name,
            url: None,
            version: self.version,
            additional_data,
        };

        let package = PackageConfig {
            shared_dir: Path::new("shared").to_owned(),
            dependencies_dir: Path::new("extern").to_owned(),
            info: package_info,
            dependencies: Default::default(),
            workspace: Default::default(),
            ..Default::default()
        };

        package.write(".")?;
        Ok(())
    }
}
