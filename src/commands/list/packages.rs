use crate::context::QpmContext;
use crate::terminal::colors::QPMColor;
use clap::Args;
use color_eyre::Result;
use itertools::Itertools;

use crate::{commands::Command, repository::Repository};

#[derive(Args, Debug, Clone)]
pub struct PackageListCommand {}

impl Command for PackageListCommand {
    fn execute(self, ctx: &QpmContext) -> Result<()> {
        let ids = ctx
            .repositories()?
            .get_package_names()?
            .into_iter()
            .sorted_by(|a, b| a.0.cmp(&b.0))
            .collect_vec();
        if !ids.is_empty() {
            println!(
                "Found {} packages on qpackages.com",
                ids.len().value_color()
            );

            ids.chunks(5).for_each(|_id| println!("{_id:?}\n"));
        } else {
            println!("qpackages.com returned 0 packages, is something wrong?");
        }
        Ok(())
    }
}
