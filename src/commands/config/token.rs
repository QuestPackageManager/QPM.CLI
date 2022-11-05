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
        if self.delete && get_keyring().get_password().is_ok() {
            get_keyring()
                .delete_password()
                .context("Removing password failed")?;
            println!("Deleted github token from config, it will no longer be used");
            return Ok(());
        } else if self.delete {
            println!("There was no github token configured, did not delete it");
            return Ok(());
        }

        match self.token {
            Some(token) => {
                // write token
                get_keyring()
                    .set_password(&token)
                    .context("Storing token failed!")?;
                println!("Configured a github token! This will now be used in qpm restore");
            }
            None => {
                    // read token, possibly unused so prepend with _ to prevent warnings
                    if let Ok(_token) = get_keyring().get_password() {
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
