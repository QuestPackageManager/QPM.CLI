use clap::Args;
use color_eyre::eyre::Context;
use itertools::Itertools;
use owo_colors::OwoColorize;
use qpm_package::models::{
    package::PackageConfig,
    triplet::{self, TripletId},
};

use crate::{
    models::package::PackageConfigExtensions,
    repository::{self},
    resolver::dependency::resolve,
    terminal::colors::QPMColor,
};

use super::Command;

#[derive(Args)]
pub struct CollapseCommand {
    #[clap(long, default_value = "false")]
    offline: bool,

    #[clap(long, short)]
    pub triplet: Option<String>,
}

impl Command for CollapseCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        let repo = repository::useful_default_new(self.offline)?;
        match self.triplet {
            Some(triplet) => {
                let triplet_id = TripletId(triplet);

                list_triplet_dependencies(package, &repo, &triplet_id)?
            }
            None => {
                println!("Listing dependencies for all triplets");
                for triplet in package
                    .triplets
                    .iter_triplets()
                    .sorted_by(|a, b| a.0.cmp(&b.0))
                {
                    println!(
                        "Listing dependencies for triplet {}",
                        triplet.0.triplet_id_color()
                    );
                    list_triplet_dependencies(package.clone(), &repo, &triplet.0).with_context(
                        || {
                            format!(
                                "Failed to list dependencies for triplet {}",
                                triplet.0.triplet_id_color()
                            )
                        },
                    )?;
                }
            }
        }
        Ok(())
    }
}

fn list_triplet_dependencies(
    package: PackageConfig,
    repo: &impl repository::Repository,
    triplet_id: &TripletId,
) -> Result<(), color_eyre::eyre::Error> {
    let resolved = resolve(&package, repo, triplet_id)?;
    for resolved_dep in resolved.sorted_by(|a, b| a.0.id.cmp(&b.0.id)) {
        let package = &resolved_dep.0;
        let triplet = &resolved_dep.1;

        let triplet_config = resolved_dep.get_triplet_settings();

        let sum = triplet_config.dependencies.len();

        println!(
            "{} --> {}/{} ({} restored dependencies)",
            package.id.dependency_id_color(),
            package.version.version_id_color(),
            triplet.triplet_id_color(),
            sum.to_string().yellow()
        );

        for (dep_id, shared_dep) in triplet_config.dependencies.iter() {
            println!(
                " - {}: ({})",
                &dep_id.dependency_id_color(),
                &shared_dep.version_range.dependency_version_color(),
            );
        }
    }
    Ok(())
}
