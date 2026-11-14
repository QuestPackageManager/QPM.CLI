use crate::context::QpmContext;
use clap::{Args, Subcommand};

use self::packages::PackageListCommand;

use super::Command;

mod packages;
mod versions;
pub type Package = versions::PackageCommand;

#[derive(Subcommand, Debug, Clone)]

pub enum ListOption {
    /// List the available packages on qpackages.com
    Packages(PackageListCommand),
    /// List the versions for a specific package
    Versions(Package),
}

#[derive(Args, Debug, Clone)]

pub struct ListCommand {
    /// What you want to list
    #[clap(subcommand)]
    pub op: ListOption,
}

impl Command for ListCommand {
    fn execute(self, ctx: &QpmContext) -> color_eyre::Result<()> {
        match self.op {
            ListOption::Packages(p) => p.execute(ctx),
            ListOption::Versions(p) => p.execute(ctx),
        }
    }
}
