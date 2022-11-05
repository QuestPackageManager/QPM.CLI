use clap::Args;
use color_eyre::eyre::{bail, Context};
use owo_colors::OwoColorize;
use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};

use crate::{
    models::{
        config::get_publish_keyring,
        package::{PackageConfigExtensions, SharedPackageConfigExtensions},
    },
    repository::{multi::MultiDependencyRepository, qpackages::QPMRepository, Repository},
    resolver::dependency,
};

use super::Command;

#[derive(Args, Debug, Clone)]

pub struct PublishCommand {
    /// the authorization header to use for publishing, if present
    pub publish_auth: Option<String>,
}

impl Command for PublishCommand {
    fn execute(&self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        if package.info.url.is_none() {
            bail!("Package without url can not be published!");
        }

        let repo = MultiDependencyRepository::useful_default_new()?;
        let qpackages = QPMRepository::default();

        let (shared_package, resolved_deps) =
            SharedPackageConfig::resolve_from_package(package, &repo)?;

        // check if all dependencies are available off of qpackages
        for dependency in resolved_deps {
            match qpackages
                .get_package(&dependency.config.info.id, &dependency.config.info.version)?
            {
                Option::Some(_s) => {}
                Option::None => {
                    bail!(
                        "dependency {} was not available on qpackages in the given version range",
                        &dependency.config.info.id
                    );
                }
            };
        }

        // check if all required dependencies are in the restored dependencies, and if they satisfy the version ranges
        for dependency in package.dependencies.iter() {
            // if we can not find any dependency that matches ID and version satisfies given range, then we are missing a dep
            if let Some(el) = shared_package
                .restored_dependencies
                .iter()
                .find(|el| el.dependency.id == dependency.id)
            {
                // if version doesn't match range, panic
                if !dependency.version_range.matches(&el.version) {
                    panic!(
                        "Restored dependency {} version ({}) does not satisfy stated range ({})",
                        dependency.id.bright_red(),
                        el.version.to_string().bright_green(),
                        dependency.version_range.to_string().bright_blue()
                    );
                }
            }
        }

        // check if url is set to download headers
        if package.info.url.is_none() {
            bail!("info.url is null, please make sure to init this with the base link to your repo, e.g. '{}'", "https://github.com/RedBrumbler/QuestPackageManager-Rust".bright_yellow());
        }
        // check if this is header only, if it's not header only check if the so_link is set, if not, panic
        if !package
            
            .info
            .additional_data
            .headers_only
            .unwrap_or(false)
            && package.info.additional_data.so_link.is_none()
        {
            bail!("soLink is not set in the package config, but this package is not header only, please make sure to either add the soLink or to make the package header only.");
        }

        // TODO: Implement a check that gets the repo and checks if the shared folder and subfolder exists, if not it throws an error and won't let you publish

        if let Some(key) = &self.publish_auth {
            QPMRepository::publish_package(&shared_package, &key)?;
        } else {
            // Empty strings are None, you shouldn't be able to publish with a None
            let publish_key = get_publish_keyring();
            QPMRepository::publish_package(
                &shared_package,
                &publish_key
                    .get_password()
                    .context("Unable to get stored publish key!")?,
            );
        }

        println!(
            "Package {} v{} published!",
            package.info.id, package.info.version
        );

        Ok(())
    }
}
