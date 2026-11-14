use crate::terminal::colors::QPMColor;
use clap::{Args, Subcommand};
use color_eyre::Result;

use crate::models::config::UserConfig;

#[derive(Subcommand, Debug, Clone)]
pub enum SymlinkOperation {
    /// Enable symlink usage
    Enable,
    /// Disable symlink usage
    Disable,
}

#[derive(Args, Debug, Clone)]

pub struct Symlink {
    #[clap(subcommand)]
    pub op: Option<SymlinkOperation>,
}

impl Symlink {
    pub fn execute(self, config: &mut UserConfig) -> Result<()> {
        // value is given
        match self.op {
            Some(symlink) => match symlink {
                SymlinkOperation::Enable => {
                    set_symlink_usage(config, true);
                }
                SymlinkOperation::Disable => {
                    set_symlink_usage(config, false);
                }
            },
            None => match config.symlink {
                Some(symlink) => {
                    println!(
                        "Current configured symlink usage is set to: {}",
                        symlink.value_color()
                    );
                }
                None => println!("Symlink usage is not configured!"),
            },
        };

        Ok(())
    }
}

fn set_symlink_usage(config: &mut UserConfig, value: bool) {
    println!("Set symlink usage to {}", value.value_color());
    config.symlink = Some(value);
}
