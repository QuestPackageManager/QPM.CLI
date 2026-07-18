use clap::Args;
use qpm_package::models::{package::PackageConfig, shared_package::SharedPackageConfig};
use semver::Version;

use crate::{commands::Command, models::package::PackageConfigExtensions};

#[derive(Args, Debug, Clone)]

pub struct EditArgs {
    ///Edit the version property of the package
    #[clap(long)]
    pub version: Option<Version>,

    #[clap(long, default_value = "false")]
    offline: bool,
}

impl Command for EditArgs {
    fn execute(self) -> color_eyre::Result<()> {
        let mut package = PackageConfig::read(".")?;
        let mut any_changed = false;
        if let Some(version) = self.version {
            package_set_version(&mut package, version);
            any_changed = true;
        }

        if any_changed {
            package.write(".")?;
            let mut shared_package = SharedPackageConfig::read(".")?;
            shared_package.config = package;
            shared_package.write(".")?;
        }
        Ok(())
    }
}

fn package_set_version(package: &mut PackageConfig, version: Version) {
    println!("Setting package version: {version}");
    package.version = version;
}
