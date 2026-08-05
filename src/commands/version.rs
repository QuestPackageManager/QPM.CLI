use clap::{Args, Subcommand};
use color_eyre::Result;

use crate::{
    services::qpm_version::{self, UpdateTarget},
    terminal::colors::QPMColor,
};

use super::Command;

#[derive(Args, Clone, Debug)]
pub struct VersionCommand {
    #[clap(subcommand)]
    op: VersionOperation,
}

#[derive(Subcommand, Debug, Clone, PartialEq, PartialOrd)]
enum VersionOperation {
    #[clap(alias("latest"))]
    Check(LatestOperationArgs),
    Current,
    Update(LatestOperationArgs),
}

#[derive(Args, Clone, Debug, PartialEq, PartialOrd)]
struct LatestOperationArgs {
    #[clap(long, short)]
    branch: Option<String>,
}

impl Command for VersionCommand {
    fn execute(self) -> Result<()> {
        match self.op {
            VersionOperation::Check(b) => {
                let current = qpm_version::current_version();
                let input_branch = b.branch.unwrap_or_else(|| current.branch.to_string());

                println!(
                    "Running branch {}@{}",
                    current.branch.dependency_version_color(),
                    current.commit.version_id_color()
                );

                let result = qpm_version::check_version(&input_branch)?;

                println!(
                    "The latest branch {input_branch} commit is {}",
                    result.latest_commit.alternate_dependency_version_color()
                );

                if result.up_to_date {
                    println!("Using the latest version");
                    return Ok(());
                }

                println!(
                    "Current QPM-RS build is behind {} commits",
                    result.behind_by.version_id_color()
                );
                println!("Changelog:");

                for message in result.changelog {
                    println!("- {message}");
                }
            }

            VersionOperation::Current => {
                let current = qpm_version::current_version();
                println!("{}@{}", current.branch, current.commit)
            }

            VersionOperation::Update(u) => {
                let current = qpm_version::current_version();

                println!(
                    "Running branch {}@{}",
                    current.branch.dependency_version_color(),
                    current.commit.version_id_color()
                );

                let download_url = match qpm_version::resolve_update_url(u.branch.as_deref())? {
                    UpdateTarget::AlreadyLatest => {
                        println!("Already running commit");
                        return Ok(());
                    }
                    UpdateTarget::DownloadUrl(url) => url,
                };

                println!("Downloading {download_url}");
                qpm_version::apply_update(&download_url)?;
                println!("Finished updating")
            }
        }
        Ok(())
    }
}
