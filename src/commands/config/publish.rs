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
        if self.delete
            && get_publish_keyring()
                .and_then(|e| e.get_password().ok())
                .is_some()
        {
            if let Some(entry) = get_publish_keyring() {
                entry
                    .delete_credential()
                    .context("Removing publish key failed")?;
            }
            println!("Deleted publish key from config, it will no longer be used");
            return Ok(());
        } else if self.delete {
            println!("There was no publish key configured, did not delete it");
            return Ok(());
        }

        if let Some(key) = self.key {
            // write key
            if let Some(entry) = get_publish_keyring() {
                entry
                    .set_password(&key)
                    .context("Failed to set publish key")?;
            } else {
                return Err(color_eyre::eyre::eyre!("Keyring unavailable"));
            }
            println!(
                "Configured a publish key! This will now be used for future qpm publish calls"
            );
        } else {
            // read token, possibly unused so prepend with _ to prevent warnings
            if let Some(key) = get_publish_keyring().and_then(|e| e.get_password().ok()) {
                #[cfg(debug_assertions)]
                println!("Configured publish key: {}", key.bright_yellow());
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
