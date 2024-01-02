use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Cursor, Read, Write},
};

use clap::{Args, Subcommand};
use color_eyre::{eyre::bail, Help};
use itertools::Itertools;
use owo_colors::OwoColorize;
use zip::ZipArchive;

use crate::{
    network::{agent::download_file_report, github},
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
    fn execute(self) -> color_eyre::Result<()> {
        match self.op {
            VersionOperation::Check(b) => {
                let base_branch = env!("VERGEN_GIT_BRANCH");
                let base_commit = env!("VERGEN_GIT_SHA");

                let input_branch = b.branch.unwrap_or(env!("VERGEN_GIT_BRANCH").to_string());
                let latest_branch = github::get_github_branch(&input_branch)?;

                println!(
                    "Running branch {}@{}",
                    base_branch.dependency_version_color(),
                    base_commit.version_id_color()
                );
                println!(
                    "The latest branch {input_branch} commit is {}",
                    latest_branch
                        .commit
                        .sha
                        .alternate_dependency_version_color()
                );

                if latest_branch.commit.sha == base_commit {
                    println!("Using the latest version");
                    return Ok(());
                }

                let diff: github::GithubCommitDiffResponse =
                    github::get_github_commit_diff(base_commit, &input_branch)?;

                if diff.behind_by > 0 {
                    bail!("Selected an older branch")
                }

                println!(
                    "Current QPM-RS build is behind {} commits",
                    diff.behind_by.version_id_color()
                );
                println!("Changelog:");

                for commit in diff.commits {
                    println!("- {}", commit.commit.message);
                }
            }

            VersionOperation::Current => {
                println!("{}@{}", env!("VERGEN_GIT_BRANCH"), env!("VERGEN_GIT_SHA"))
            }
            VersionOperation::Update(u) => {
                let base_branch = env!("VERGEN_GIT_BRANCH");
                let base_commit = env!("VERGEN_GIT_SHA");

                let input_branch = u.branch.unwrap_or(env!("VERGEN_GIT_BRANCH").to_string());
                let latest_branch = github::get_github_branch(&input_branch)?;

                println!(
                    "Running branch {}@{}",
                    base_branch.dependency_version_color(),
                    base_commit.version_id_color()
                );

                if base_commit == latest_branch.commit.sha {
                    println!("Already running commit");
                    return Ok(());
                }

                println!(
                    "Downloading {}",
                    latest_branch
                        .commit
                        .sha
                        .alternate_dependency_version_color()
                );

                let path = env::current_exe()?;
                let tmp_path = path.with_extension("tmp");
                let zip_bytes = download_file_report(
                    &github::download_github_artifact_url(&input_branch),
                    |_, _| {},
                )?;

                let cursor = Cursor::new(zip_bytes);
                let mut zip = ZipArchive::new(cursor)?;
                let bytes = zip.by_index(0)?.bytes();

                println!("Finished downloading, writing to temp file");
                let tmp_file = File::create(&tmp_path)?;
                let perms = fs::metadata(&path)?.permissions();
                fs::set_permissions(&tmp_path, perms)?;

                let mut buf_tmp_write = BufWriter::new(tmp_file);

                buf_tmp_write.write_all(&bytes.try_collect::<_, Vec<u8>, _>()?)?;

                let suggestion = format!(
                    "Try renaming manually.\nmv \"{}\" \"{}\" {}",
                    tmp_path.to_str().unwrap().red(),
                    path.to_str().unwrap().blue(),
                    if cfg!(windows) { "-Force" } else { "" }
                );

                println!("Renaming tmp file");
                fs::rename(&path, path.with_extension("old")).suggestion(suggestion.clone())?;
                fs::rename(&tmp_path, &path).suggestion(suggestion)?;
                println!("Finished updating")
            }
        }
        Ok(())
    }
}
