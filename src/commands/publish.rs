use std::io::Cursor;

use bytes::{BufMut, BytesMut};
use clap::{Args, ValueEnum};
use color_eyre::eyre::{Context, ContextCompat, bail};
use owo_colors::OwoColorize;
use qpm_package::models::{
    package::PackageConfig,
    qpackages::QPackagesPackage,
    qpkg,
    shared_package::{SharedPackageConfig, SharedTriplet},
    triplet::TripletId,
};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::{
    models::{config::get_publish_keyring, package::PackageConfigExtensions},
    network::agent::download_file_report,
    repository::{Repository, qpackages::QPMRepository},
    terminal::colors::QPMColor,
    utils::json,
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
        let published = match self.backend {
            Backend::QPackages => self.qpackages_publish()?,
        };

        println!(
            "Package {} v{} published!",
            published.config.id.dependency_id_color(),
            published.config.version.version_id_color()
        );

        Ok(())
    }
}

impl PublishCommand {
    fn validate_qpkg(
        &self,
        package: &PackageConfig,
    ) -> Result<Cursor<BytesMut>, color_eyre::eyre::Error> {
        let mut bytes = BytesMut::new().writer();
        // TODO: What if the URL is not accessible due to Authorization or rate limits?
        download_file_report(&self.qpkg_url, &mut bytes, |_, _| {}).context(
            "Downloading qpkg file failed. QPKG URL must be accessible at time of publishing",
        )?;

        let mut cursor = Cursor::new(bytes.into_inner());

        // validate config in QPKG matches
        {
            let mut qpkg = ZipArchive::new(&mut cursor).context("Failed to read qpkg zip")?;
            let qpkg_config = qpkg
                .by_name("config.json")
                .context("Failed to find config.json in qpkg")?;
            let qpkg_config: PackageConfig = json::json_from_reader_fast(qpkg_config)
                .context("Failed to parse config.json in qpkg")?;

            if qpkg_config != *package {
                bail!(
                    "QPKG config mismatch. Expected{:#?}\nGot {:#?}",
                    package,
                    qpkg_config
                );
            }
        }

        Ok(cursor)
    }

    fn qpackages_publish(self) -> Result<QPackagesPackage, color_eyre::eyre::Error> {
        let package = PackageConfig::read(".")?;
        let qpackages = QPMRepository::default();

        let shared_package = SharedPackageConfig::read(".")?;

        // validate current package against shared package
        for (triplet_id, shared_triplet) in &shared_package.locked_triplet {
            check_triplet(&package, &qpackages, triplet_id, shared_triplet)
                .with_context(|| format!("Triplet {triplet_id}"))?;
        }

        let qpkg_cursor = self
            .validate_qpkg(&package)
            .with_context(|| "Validating QPKG failed")?;

        // checksum verify
        let result = Sha256::digest(qpkg_cursor.get_ref());
        let checksum = hex::encode(result);

        let qpackage = QPackagesPackage {
            config: package,
            qpkg_checksum: Some(checksum),
            qpkg_url: self.qpkg_url,
        };

        if let Some(key) = &self.publish_auth {
            QPMRepository::publish_package(&qpackage, key)?;
        } else {
            // Empty strings are None, you shouldn't be able to publish with a None
            let publish_key = get_publish_keyring();
            QPMRepository::publish_package(
                &qpackage,
                &publish_key
                    .get_password()
                    .context("Unable to get stored publish key!")?,
            )?;
        }
        Ok(qpackage)
    }
}

fn check_triplet(
    package: &PackageConfig,
    repo: &impl Repository,
    triplet_id: &TripletId,
    shared_triplet: &SharedTriplet,
) -> Result<(), color_eyre::eyre::Error> {
    let triplet = package
        .triplets
        .get_triplet_settings(triplet_id)
        .context("Failed to get triplet settings")?;
    let resolved_deps = &shared_triplet.restored_dependencies;
    for (dep_id, dep) in resolved_deps {
        if repo.get_package(dep_id, &dep.restored_version)?.is_none() {
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
