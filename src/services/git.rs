use std::{
    io::BufReader,
    path::Path,
    process::{Command, Stdio},
};

use color_eyre::{
    Section,
    eyre::{Context, Result, bail},
};
use owo_colors::OwoColorize;

pub fn check_git() -> color_eyre::Result<()> {
    let mut git = std::process::Command::new("git");
    git.arg("--version");

    match git.output() {
        Ok(_) => {
            #[cfg(debug_assertions)]
            println!("git detected on command line!");
            Ok(())
        }
        Err(_e) => {
            #[cfg(windows)]
            bail!(
                "Please make sure git ({}) is installed and on path, then try again!",
                "https://git-scm.com/download/win".bright_yellow()
            );
            #[cfg(target_os = "linux")]
            bail!(
                "Please make sure git ({}) is installed and on path, then try again!",
                "https://git-scm.com/download/linux".bright_yellow()
            );
            #[cfg(target_os = "macos")]
            bail!(
                "Please make sure git ({}) is installed and on path, then try again!",
                "https://git-scm.com/download/mac".bright_yellow()
            );
        }
    }
}

/// Clones `url` (optionally at `branch`) into `out`. If `auth_token` is given, it's inserted
/// into the URL for github.com clones and scrubbed from any error output.
pub fn clone(
    mut url: String,
    branch: Option<&String>,
    out: &Path,
    auth_token: Option<&str>,
) -> Result<bool> {
    check_git()?;
    if let Some(token) = auth_token
        && let Some(gitidx) = url.find("github.com")
    {
        url.insert_str(gitidx, &format!("{token}@"));
    }

    if url.ends_with('/') {
        url = url[..url.len() - 1].to_string();
    }

    let mut git = Command::new("git");
    git.arg("clone")
        .arg(format!("{url}.git"))
        .arg(out)
        .arg("--depth")
        .arg("1")
        .arg("--recurse-submodules")
        .arg("--shallow-submodules")
        .arg("--single-branch")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(branch_unwrapped) = branch {
        git.arg("-b").arg(branch_unwrapped);
    } else {
        println!("No branch name found, cloning default branch");
    }

    let mut child = git
        .spawn()
        .context("Git clone package")
        .with_suggestion(|| format!("File a bug report. Used the following command: {git:#?}"))?;

    match child.wait() {
        Ok(e) => {
            if e.code().unwrap_or(-1) != 0 {
                let stderr = BufReader::new(child.stderr.as_mut().unwrap());

                let mut error_string = std::str::from_utf8(stderr.buffer())?.to_string();

                if let Some(token) = auth_token {
                    error_string = error_string.replace(token, "***");
                }

                bail!("Exit code {}: {}", e, error_string);
            }
        }
        Err(e) => {
            let mut error_string = e.to_string();

            if let Some(token) = auth_token {
                error_string = error_string.replace(token, "***");
            }

            bail!("{}", error_string);
        }
    }

    Ok(out.try_exists()?)
}
