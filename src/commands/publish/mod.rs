use clap::{Args, ValueEnum};
use color_eyre::eyre::{Context, ContextCompat, bail};
use owo_colors::OwoColorize;
use qpm_package::models::{
    package::PackageConfig,
    shared_package::{SharedPackageConfig, SharedTriplet},
    triplet::TripletId,
};

use crate::{
    models::{config::get_publish_keyring, package::PackageConfigExtensions},
    repository::{Repository, qpackages::QPMRepository},
    terminal::colors::QPMColor,
};

use super::Command;

#[derive(ValueEnum, Debug, Clone)]
enum Backend {
    QPackages,
}

#[derive(Args, Debug, Clone)]

pub struct PublishCommand {
    /// The url to the qpkg
    pub qpkg_url: String,

    #[clap(long, default_value = "qpackages")]
    backend: Backend,

    /// the authorization header to use for publishing, if present
    #[clap(long = "token")]
    pub publish_auth: Option<String>,
}

impl Command for PublishCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;

        let qpackages = QPMRepository::default();

        let shared_package = SharedPackageConfig::read(".")?;

        for (triplet_id, shared_triplet) in &shared_package.locked_triplet {
            check_triplet(&package, &qpackages, triplet_id, shared_triplet)
                .with_context(|| format!("Triplet {triplet_id}"))?;
        }

        if let Some(key) = &self.publish_auth {
            QPMRepository::publish_package(&shared_package, key)?;
        } else {
            // Empty strings are None, you shouldn't be able to publish with a None
            let publish_key = get_publish_keyring();
            QPMRepository::publish_package(
                &shared_package,
                &publish_key
                    .get_password()
                    .context("Unable to get stored publish key!")?,
            )?;
        }

        println!(
            "Package {} v{} published!",
            shared_package.config.id.dependency_id_color(),
            shared_package.config.version.version_id_color()
        );

        Ok(())
    }
}

fn check_triplet(
    package: &PackageConfig,
    qpackages: &QPMRepository,
    triplet_id: &TripletId,
    shared_triplet: &SharedTriplet,
) -> Result<(), color_eyre::eyre::Error> {
    let triplet = package
        .triplets
        .get_triplet_settings(triplet_id)
        .context("Failed to get triplet settings")?;
    let resolved_deps = &shared_triplet.restored_dependencies;
    for (dep_id, dep) in resolved_deps {
        if qpackages
            .get_package(dep_id, &dep.restored_version)?
            .is_none()
        {
            bail!(
                "dependency {} was not available on qpackages in the given version range",
                &dep_id.dependency_id_color()
            );
        };
    }
    for (dep_id, dependency) in &triplet.dependencies {
        // if we can not find any dependency that matches ID and version satisfies given range, then we are missing a dep
        let el = shared_triplet
            .restored_dependencies
            .get(dep_id)
            .context(format!(
                "Dependency {} not found in restored dependencies",
                dep_id.dependency_id_color()
            ))?;

        // if version doesn't match range, panic
        if !dependency.version_range.matches(&el.restored_version) {
            bail!(
                "Restored dependency {} version ({}) does not satisfy stated range ({})",
                dep_id.dependency_id_color(),
                el.restored_version.red(),
                dependency.version_range.green()
            );
        }
    }

    // check if url is set to download headers

    Ok(())
}
