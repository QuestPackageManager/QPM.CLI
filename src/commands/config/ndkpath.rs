use crate::terminal::colors::QPMColor;
use clap::Args;
use color_eyre::Result;

use crate::models::config::UserConfig;

#[derive(Args, Debug, Clone)]
pub struct NDKPath {
    /// The path to set for the ndk path
    pub ndk_path: Option<String>,
}

impl NDKPath {
    pub fn execute(self, config: &mut UserConfig) -> Result<()> {
        match self.ndk_path {
            Some(path) => {
                println!("Set ndk path to {}!", path.file_path_color());
                config.ndk_download_path = Some(path.into());
            }
            None => match &config.ndk_download_path {
                Some(path) => println!(
                    "Current configured ndk path is: {}",
                    path.to_str().unwrap().file_path_color()
                ),
                None => println!("No ndk path was configured!"),
            },
        }
        Ok(())
    }
}
