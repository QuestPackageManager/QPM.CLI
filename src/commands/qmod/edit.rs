use std::path::Path;

use clap::Args;
use color_eyre::eyre::ContextCompat;
use qpm_package::models::{package::PackageConfig, shared_package::SharedPackageConfig};
use qpm_qmod::models::mod_json::ModJson;
use semver::Version;

use crate::{
    commands::Command,
    models::{mod_json::ModJsonExtensions, package::PackageConfigExtensions},
};

/// Some properties are not editable through the qmod edit command, these properties are either editable through the package, or not at all
#[derive(Args, Debug, Clone)]

pub struct EditQmodJsonCommand {
    /// The schema version this mod was made for, ex. '0.1.1'
    #[clap(long = "qpversion")]
    pub schema_version: Option<Version>,
    /// Author of the mod, ex. 'RedBrumbler'
    #[clap(long)]
    pub author: Option<String>,
    /// Optional slot for if you ported a mod, ex. 'Fern'
    #[clap(long)]
    pub porter: Option<String>,
    /// id of the package the mod is for, ex. 'com.beatgames.beatsaber'
    #[clap(long = "packageID")]
    pub package_id: Option<String>,
    /// Version of the package, ex. '1.1.0'
    #[clap(long = "packageVersion")]
    pub package_version: Option<String>,
    /// description for the mod, ex. 'The best mod to exist ever!'
    #[clap(long)]
    pub description: Option<String>,
    /// optional cover image filename, ex. 'cover.png'
    #[clap(long = "coverImage")]
    pub cover_image: Option<String>,
}

impl Command for EditQmodJsonCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        let shared_package = SharedPackageConfig::read(".")?;
        let triplet = package
            .triplets
            .get_triplet_settings(&shared_package.restored_triplet)
            .context("Restored triplet not in package config")?;

        let mod_template = triplet
            .qmod_template
            .as_deref()
            .unwrap_or_else(|| Path::new("mod.template.json"));

        let mut json = ModJson::read(mod_template)?;

        if let Some(schema_version) = self.schema_version {
            json.schema_version = schema_version;
        }
        if let Some(author) = self.author {
            json.author = author;
        }
        if let Some(porter) = self.porter {
            if porter == "clear" {
                json.porter = None;
            } else {
                json.porter = Some(porter);
            }
        }
        if let Some(package_id) = self.package_id {
            json.package_id = Some(package_id);
        }
        if let Some(package_version) = self.package_version {
            json.package_version = Some(package_version);
        }
        if let Some(description) = self.description {
            if description == "clear" {
                json.description = None;
            } else {
                json.description = Some(description);
            }
        }
        if let Some(cover_image) = self.cover_image {
            if cover_image == "clear" {
                json.cover_image = None;
            } else {
                json.cover_image = Some(cover_image);
            }
        }

        json.write(mod_template)?;
        Ok(())
    }
}
