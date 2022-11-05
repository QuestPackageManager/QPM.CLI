use clap::Args;
use color_eyre::Result;
use owo_colors::OwoColorize;

use crate::models::config::UserConfig;

#[derive(Args, Debug, Clone)]
pub struct NDKPath {
    /// The path to set for the ndk path
    pub ndk_path: Option<String>,
}

impl NDKPath {
    pub fn execute(&self, config: &mut UserConfig) -> Result<()> {
        match self.ndk_path {
            Some(path) => {
                println!("Set ndk path to {}!", path.bright_yellow());
                config.ndk_path = Some(path);
            }
            None => {
                match config.ndk_path {
                    Some(path) =>         println!("Current configured ndk path is: {}", path.bright_yellow()),
                    None => println!("No ndk path was configured!"),
                }
            },
        }
        Ok(())
    }
}


