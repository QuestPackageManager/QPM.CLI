use crate::context::QpmContext;
use crate::terminal::colors::QPMColor;
use clap::Args;
use qpm_package::models::package::DependencyId;

use crate::{commands::Command, repository::Repository};

#[derive(Args, Debug, Clone)]
pub struct PackageCommand {
    pub package: String,
    #[clap(short, long)]
    pub latest: bool,
}

impl Command for PackageCommand {
    fn execute(self, ctx: &QpmContext) -> color_eyre::Result<()> {
        let versions = ctx
            .repositories()?
            .get_package_versions(&DependencyId(self.package.clone()))?;

        match versions {
            Some(package_versions) => {
                if self.latest {
                    println!(
                        "The latest version for package {} is {}",
                        self.package.dependency_id_color(),
                        package_versions.first().unwrap().version_id_color()
                    );

                    return Ok(());
                }

                println!(
                    "Package {} has {} versions on qpackages.com:",
                    self.package.dependency_id_color(),
                    package_versions.len().value_color()
                );
                for package_version in package_versions.iter().rev() {
                    println!(" - {}", package_version.version_id_color());
                }
            }
            _ => {
                println!(
                    "Package {} either did not exist or has no versions on qpackages.com",
                    self.package.dependency_id_color()
                );
            }
        }
        Ok(())
    }
}
