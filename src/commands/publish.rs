use clap::{Args, ValueEnum};
use color_eyre::eyre::ContextCompat;
use qpm_package::models::{package::PackageConfig, shared_package::SharedPackageConfig};

use crate::{
    models::{config::get_publish_keyring, package::PackageConfigExtensions},
    repository::qpackages::QPMRepository,
    services::publish::PackagePublisher,
    terminal::colors::QPMColor,
};

use super::Command;

#[derive(ValueEnum, Debug, Clone)]
enum Backend {
    #[clap(name = "qpackages")]
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
        let shared_package = SharedPackageConfig::read(".")?;
        let qpackages = QPMRepository::default();

        let publisher = PackagePublisher::validate(
            package,
            &shared_package,
            self.qpkg_url.clone(),
            &qpackages,
        )?;

        let auth_token = match &self.publish_auth {
            Some(key) => key.clone(),
            // Empty strings are None, you shouldn't be able to publish with a None
            None => get_publish_keyring()
                .and_then(|p| p.get_password().ok())
                .context("Unable to get stored publish key!")?,
        };

        let published = match self.backend {
            Backend::QPackages => publisher.publish_to_qpackages(&auth_token)?,
        };

        println!(
            "Package {} v{} published!",
            published.config.id.dependency_id_color(),
            published.config.version.version_id_color()
        );

        Ok(())
    }
}
