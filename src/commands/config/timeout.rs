use clap::Args;
use color_eyre::Result;
use owo_colors::OwoColorize;

use crate::models::config::UserConfig;

#[derive(Args, Debug, Clone)]
pub struct Timeout {
    pub timeout: Option<u32>,
}

impl Timeout {
    pub fn execute(&self, config: &mut UserConfig) -> Result<()> {
        match self.timeout {
            Some(timeout) => {
                println!("Set timeout to {}!", timeout.bright_yellow());
                config.timeout.insert(timeout);
            }
            None => match config.timeout {
                Some(timeout) => println!(
                    "Current configured timeout is set to: {}",
                    timeout.bright_yellow()
                ),
                None => println!("Timeout is not configured!"),
            },
        }
        Ok(())
    }
}
