use clap::Args;
use color_eyre::Result;
use owo_colors::OwoColorize;

use crate::{
    commands::Command,
    repository::{multi::MultiDependencyRepository, Repository},
};

#[derive(Args, Debug, Clone)]
pub struct PackageListCommand {}

impl Command for PackageListCommand {
    fn execute(self) -> Result<()> {
        let ids = MultiDependencyRepository::useful_default_new()?.get_package_names()?;
        if !ids.is_empty() {
            println!(
                "Found {} packages on qpackages.com",
                ids.len().bright_yellow()
            );

            ids.chunks(5).for_each(|_id| println!("{:?}\n", ids));
        } else {
            println!("qpackages.com returned 0 packages, is something wrong?");
        }
        Ok(())
    }
}
