use clap::{Args, Subcommand};

mod cache;
mod ndkpath;
mod publish;
mod symlink;
mod timeout;
mod token;

use owo_colors::OwoColorize;

use crate::models::config::UserConfig;

use super::Command;

#[derive(Args, Debug, Clone)]

pub struct ConfigCommand {
    /// The operation to execute
    #[clap(subcommand)]
    pub op: ConfigOperation,
    /// use this flag to edit the local config instead of the global one
    #[clap(short, long)]
    pub local: bool,
}

#[derive(Subcommand, Debug, Clone)]

pub enum ConfigOperation {
    /// Get or set the cache path
    Cache(cache::CacheCommand),
    /// Enable or disable symlink usage
    Symlink(symlink::Symlink),
    /// Get or set the timeout for web requests
    Timeout(timeout::Timeout),
    /// Get or set the github token used for restore
    Token(token::TokenCommand),
    /// Print the location of the global config
    Location,
    /// Get or set the ndk path used in generation of build files
    NDKPath(ndkpath::NDKPath),
    /// Get or set the publish key used for publish
    Publish(publish::KeyCommand),
}

impl Command for ConfigCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let mut config = if self.local {
            UserConfig::read_workspace()?.unwrap()
        } else {
            UserConfig::read_global()?
        };

        match self.op {
            ConfigOperation::Cache(c) => c.execute(&mut config)?,
            ConfigOperation::Symlink(s) => s.execute(&mut config)?,
            ConfigOperation::Timeout(t) => t.execute(&mut config)?,
            ConfigOperation::Token(t) => t.execute()?,
            ConfigOperation::Location => println!(
                "Global Config is located at {}",
                UserConfig::global_config_path().display().bright_yellow()
            ),
            ConfigOperation::NDKPath(p) => p.execute(&mut config)?,
            ConfigOperation::Publish(k) => k.execute()?,
        };

        config.write(self.local)?;
        Ok(())
    }
}
