use clap::{Args, Subcommand};
use color_eyre::eyre::ContextCompat;
use qpm_package::models::{
    package::PackageConfig,
    shared_package::SharedPackageConfig,
    triplet::{PackageTriplet, TripletId},
};

use crate::{
    commands::Command,
    models::package::PackageConfigExtensions,
    repository::{self},
    utils::toggle::Toggle,
};

#[derive(Args, Debug, Clone)]

pub struct EditExtraArgs {
    /// Provide a link to the mod
    #[clap(long = "modLink")]
    pub mod_link: Option<String>,

    /// Provide a qmod id to set the extra for
    #[clap(long = "qmodId")]
    pub qmod_id: Option<String>,

    /// The triplet to edit the extra for
    #[clap(long, short)]
    pub triplet: String,

    #[clap(long, default_value = "false")]
    offline: bool,
}

impl Command for EditExtraArgs {
    fn execute(self) -> color_eyre::Result<()> {
        let mut package = PackageConfig::read(".")?;
        let triplet = package
            .triplets
            .get_triplet_mut(&TripletId(self.triplet))
            .context("Failed to get triplet settings")?;

        let mut any_changed = false;

        if let Some(mod_link) = self.mod_link {
            println!("Setting mod_link: {mod_link:#?}");
            triplet.qmod_url = Some(mod_link);
            any_changed = true;
        }

        if let Some(qmod_id) = self.qmod_id {
            println!("Setting qmod_id: {qmod_id:#?}");
            triplet.qmod_id = Some(qmod_id);
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
