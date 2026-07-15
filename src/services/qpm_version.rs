use std::{
    env,
    fs::{self, File},
    io::{BufReader, BufWriter, Cursor, Read, Write},
};

use bytes::{BufMut, BytesMut};
use color_eyre::{Help, Result, eyre::bail};
use itertools::Itertools;
use owo_colors::OwoColorize;
use zip::ZipArchive;

use crate::services::{network::download_file_report, github};

/// The branch and commit qpm2 was built from
pub struct CurrentVersion {
    pub branch: &'static str,
    pub commit: &'static str,
}

pub fn current_version() -> CurrentVersion {
    CurrentVersion {
        branch: env!("VERGEN_GIT_BRANCH"),
        commit: env!("VERGEN_GIT_SHA"),
    }
}

pub struct VersionCheck {
    pub latest_commit: String,
    pub up_to_date: bool,
    pub behind_by: i32,
    pub changelog: Vec<String>,
}

/// Compares the running build against `branch`'s latest commit on GitHub
pub fn check_version(branch: &str) -> Result<VersionCheck> {
    let current = current_version();
    let latest_branch = github::get_github_branch(branch)?;

    if latest_branch.commit.sha == current.commit {
        return Ok(VersionCheck {
            latest_commit: latest_branch.commit.sha,
            up_to_date: true,
            behind_by: 0,
            changelog: Vec::new(),
        });
    }

    let diff = github::get_github_commit_diff(current.commit, branch)?;

    if diff.behind_by > 0 {
        bail!("Selected an older branch")
    }

    Ok(VersionCheck {
        latest_commit: latest_branch.commit.sha,
        up_to_date: false,
        behind_by: diff.behind_by,
        changelog: diff.commits.into_iter().map(|c| c.commit.message).collect(),
    })
}

pub enum UpdateTarget {
    /// The requested branch's commit matches the running build; nothing to do
    AlreadyLatest,
    DownloadUrl(String),
}

/// Resolves the artifact URL to update to: the given branch's latest build, or the
/// bleeding release if no branch was given.
pub fn resolve_update_url(branch: Option<&str>) -> Result<UpdateTarget> {
    let current = current_version();

    match branch {
        Some(input_branch) => {
            let latest_branch = github::get_github_branch(input_branch)?;

            if current.commit == latest_branch.commit.sha {
                return Ok(UpdateTarget::AlreadyLatest);
            }

            Ok(UpdateTarget::DownloadUrl(
                github::download_github_artifact_url(input_branch),
            ))
        }
        None => Ok(UpdateTarget::DownloadUrl(
            github::bleeding_release_github_artifact_url(),
        )),
    }
}

/// Downloads the qpm2 build at `download_url` and replaces the currently running executable
/// with it, keeping the previous binary around with an `.old` extension.
pub fn apply_update(download_url: &str) -> Result<()> {
    let path = env::current_exe()?;
    let tmp_path = path.with_extension("tmp");
    let mut bytes = BytesMut::with_capacity(1024 * 1024 * 10).writer();
    download_file_report(download_url, &mut bytes, |_, _| {})?;

    let cursor = Cursor::new(bytes.into_inner());
    let mut zip = ZipArchive::new(cursor)?;
    let buf_reader = BufReader::new(zip.by_index(0)?);
    let bytes = buf_reader.bytes();

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

    Ok(())
}
