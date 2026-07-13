use clap::Args;
use qpm_package::models::{package::PackageConfig, shared_package::SharedPackageConfig};

use crate::{commands::Command, models::package::PackageConfigExtensions};

#[derive(Args, Debug, Clone)]

pub struct EditExtraArgs {
    /// Provide a link to the mod
    #[clap(long = "modLink")]
    pub mod_link: Option<String>,

    /// Provide a qmod id to set the extra for
    #[clap(long = "qmodId")]
    pub qmod_id: Option<String>,

    #[clap(long, default_value = "false")]
    offline: bool,
}

impl Command for EditExtraArgs {
    fn execute(self) -> color_eyre::Result<()> {
        let mut package = PackageConfig::read(".")?;

        let mut any_changed = false;

        if let Some(mod_link) = self.mod_link {
            println!("Setting mod_link: {mod_link:#?}");
            package.qmod.download_url = Some(mod_link);
            any_changed = true;
        }

        if let Some(qmod_id) = self.qmod_id {
            println!("Setting qmod_id: {qmod_id:#?}");
            package.qmod.id = Some(qmod_id);
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
