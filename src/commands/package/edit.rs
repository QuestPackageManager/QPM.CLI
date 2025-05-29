use clap::Args;
use qpm_package::models::{package::{DependencyId, PackageConfig}, shared_package::SharedPackageConfig};
use semver::Version;

use crate::{
    commands::Command,
    models::package::PackageConfigExtensions,
    repository::{self},
};

#[derive(Args, Debug, Clone)]

pub struct EditArgs {
    ///Edit the id property of the package
    #[clap(long)]
    pub id: Option<String>,
    ///Edit the name property of the package
    #[clap(long)]
    pub name: Option<String>,
    ///Edit the url property of the package
    #[clap(long)]
    pub url: Option<String>,
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
        if let Some(id) = self.id {
            package_set_id(&mut package, id);
            any_changed = true;
        }
        if let Some(name) = self.name {
            package_set_name(&mut package, name);
            any_changed = true;
        }
        if let Some(url) = self.url {
            package_set_url(&mut package, url);
            any_changed = true;
        }
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

fn package_set_id(package: &mut PackageConfig, id: String) {
    println!("Setting package id: {id}");
    package.id = DependencyId(id);
}


fn package_set_url(package: &mut PackageConfig, url: String) {
    println!("Setting package url: {url}");
    package.url = Option::Some(url);
}

fn package_set_version(package: &mut PackageConfig, version: Version) {
    println!("Setting package version: {version}");
    package.version = version;
}
