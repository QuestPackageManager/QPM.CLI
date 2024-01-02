use clap::Args;
use color_eyre::Result;
use itertools::Itertools;
use owo_colors::OwoColorize;

use crate::{
    commands::Command,
    repository::{multi::MultiDependencyRepository, Repository, self},
};

#[derive(Args, Debug, Clone)]
pub struct PackageListCommand {
    #[clap(long, default_value = "false")]
    offline: bool,
}

impl Command for PackageListCommand {
    fn execute(self) -> Result<()> {
        let ids = repository::useful_default_new(self.offline)?
            .get_package_names()?
            .into_iter()
            .sorted()
            .collect_vec();
        if !ids.is_empty() {
            println!(
                "Found {} packages on qpackages.com",
                ids.len().bright_yellow()
            );

            ids.chunks(5).for_each(|_id| println!("{_id:?}\n"));
        } else {
            println!("qpackages.com returned 0 packages, is something wrong?");
        }
        Ok(())
    }
}
