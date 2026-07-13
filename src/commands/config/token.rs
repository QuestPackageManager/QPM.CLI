use clap::Args;
use color_eyre::eyre::Context;
use owo_colors::OwoColorize;

use crate::{commands::Command, models::config::get_keyring};

#[derive(Args, Debug, Clone)]
pub struct TokenCommand {
    pub token: Option<String>,
    #[clap(long)]
    pub delete: bool,
}

impl Command for TokenCommand {
    fn execute(self) -> color_eyre::Result<()> {
        if self.delete && get_keyring().and_then(|e| e.get_password().ok()).is_some() {
            if let Some(entry) = get_keyring() {
                entry
                    .delete_credential()
                    .context("Removing password failed")?;
            }
            println!("Deleted github token from config, it will no longer be used");
            return Ok(());
        } else if self.delete {
            println!("There was no github token configured, did not delete it");
            return Ok(());
        }

        match self.token {
            Some(token) => {
                // write token
                if let Some(entry) = get_keyring() {
                    entry
                        .set_password(&token)
                        .context("Storing token failed!")?;
                } else {
                    return Err(color_eyre::eyre::eyre!("Keyring unavailable"));
                }
                println!("Configured a github token! This will now be used in qpm restore");
            }
            None => {
                // read token, possibly unused so prepend with _ to prevent warnings
                if let Some(_token) = get_keyring().and_then(|e| e.get_password().ok()) {
                    #[cfg(debug_assertions)]
                    println!("Configured github token: {}", _token.bright_yellow());
                    #[cfg(not(debug_assertions))]
                    println!(
                        "In release builds you {} view the configured github token, a token was configured though!",
                        "cannot".bright_red()
                    );
                } else {
                    println!("No token was configured, or getting the token failed!");
                }
            }
        }
        Ok(())
    }
}
