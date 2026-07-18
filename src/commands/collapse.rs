use clap::Args;
use itertools::Itertools;
use owo_colors::OwoColorize;
use qpm_package::models::package::PackageConfig;

use crate::{
    models::package::PackageConfigExtensions,
    repository::{self},
    services::restore::PackageRestorer,
    terminal::colors::QPMColor,
};

use super::Command;

#[derive(Args)]
pub struct CollapseCommand {
    #[clap(long, default_value = "false")]
    offline: bool,

    #[clap(long, short)]
    pub env: bool,
}

impl Command for CollapseCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        let repo = repository::useful_default_new(self.offline)?;

        list_dependencies(package, &repo, self.env)?;

        Ok(())
    }
}

fn list_dependencies(
    package: PackageConfig,
    repo: &impl repository::Repository,
    print_env: bool,
) -> Result<(), color_eyre::eyre::Error> {
    let restorer = PackageRestorer::resolve(package, repo)?;
    let resolved = restorer
        .resolved_deps()
        .iter()
        .sorted_by(|a, b| a.config.id.cmp(&b.config.id));
    for resolved_artifact in resolved {
        let resolved_dep = &resolved_artifact.config;
        let sum = resolved_dep.dependencies.len();

        println!(
            "{} --> {} ({} restored dependencies)",
            resolved_dep.id.dependency_id_color(),
            resolved_dep.version.version_id_color(),
            sum.to_string().yellow()
        );

        if print_env {
            println!("Environment variables:");
            for (key, value) in resolved_dep.workspace.env.iter().flatten() {
                println!(" - {}: {}", key.cyan(), value.green());
            }
        }

        for (dep_id, dep) in resolved_dep.dependencies.iter() {
            println!(
                " - {}: ({})",
                &dep_id.dependency_id_color(),
                &dep.version_range.dependency_version_color(),
            );
        }
    }
    Ok(())
}
