use clap::Args;
use color_eyre::eyre::Context;
use owo_colors::OwoColorize;

use crate::{commands::Command, models::config::get_publish_keyring};

#[derive(Args, Debug, Clone)]
pub struct KeyCommand {
    pub key: Option<String>,
    #[clap(long)]
    pub delete: bool,
}

impl Command for KeyCommand {
    fn execute(self) -> color_eyre::Result<()> {
        if self.delete && get_publish_keyring().get_password().is_ok() {
            get_publish_keyring()
                .delete_password()
                .context("Removing publish key failed")?;
            println!("Deleted publish key from config, it will no longer be used");
            return Ok(());
        } else if self.delete {
            println!("There was no publish key configured, did not delete it");
            return Ok(());
        }

        if let Some(key) = self.key {
            // write key
            get_publish_keyring()
                .set_password(&key)
                .context("Failed to set publish key")?;
            println!(
                "Configured a publish key! This will now be used for future qpm publish calls"
            );
        } else {
            // read token, possibly unused so prepend with _ to prevent warnings
            if let Ok(_key) = get_publish_keyring().get_password() {
                #[cfg(debug_assertions)]
                println!("Configured publish key: {}", _key.bright_yellow());
                #[cfg(not(debug_assertions))]
                println!(
                    "In release builds you {} view the configured publish key!",
                    "cannot".bright_red()
                );
            } else {
                println!("No publish key was configured, or getting the publish key failed!");
            }
        }
        Ok(())
    }
}
