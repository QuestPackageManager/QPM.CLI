use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
    process::{Command, Stdio},
};

use color_eyre::{
    Result, Section,
    eyre::{Context, bail},
};
use owo_colors::OwoColorize;
//use duct::cmd;
use serde::{Deserialize, Serialize};

use crate::{
    models::config::get_keyring,
    network::agent::{download_file_report, get_agent},
    terminal::colors::QPMColor,
};

pub fn check_git() -> Result<()> {
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

pub fn get_release(url: &str, out: &std::path::Path) -> Result<bool> {
    check_git()?;
    if let Ok(token_unwrapped) = get_keyring().get_password() {
        get_release_with_token(url, out, &token_unwrapped)
    } else {
        get_release_without_token(url, out)
    }
}

pub fn get_release_without_token(url: &str, out: &std::path::Path) -> Result<bool> {
    let file = File::create(out).context("create so file failed")?;
    let mut buf = BufWriter::new(file);

    download_file_report(url, &mut buf, |_, _| {}).with_context(|| {
        format!(
            "Failed while downloading {} to {}",
            url.blue(),
            out.display().file_path_color()
        )
    })?;

    Ok(out.exists())
}

pub fn get_release_with_token(url: &str, out: &std::path::Path, token: &str) -> Result<bool> {
    // had token, use it!
    // download url for a private thing: still need to get asset id!
    // from this: "https://github.com/$USER/$REPO/releases/download/$TAG/$FILENAME"
    // to this: "https://$TOKEN@api.github.com/repos/$USER/$REPO/releases/assets/$ASSET_ID"
    let split: Vec<String> = url.split('/').map(|el| el.to_string()).collect();

    // Obviously this is a bad way of parsing the GH url but like I see no better way, people better not use direct lib uploads lol
    // (I know mentioning it here will make people do that, so fuck y'all actually thinking of doing that)
    // HACK: Not ideal way of getting these values
    let user = split.get(3).unwrap();
    let repo = split.get(4).unwrap();
    let tag = split.get(7).unwrap();
    let filename = split.get(8).unwrap();

    let asset_data_link = format!(
        "https://{}@api.github.com/repos/{}/{}/releases/tags/{}",
        &token, &user, &repo, &tag
    );

    let data = match get_agent().get(asset_data_link).send() {
        Ok(o) => o.json::<GithubReleaseData>().unwrap(),
        Err(e) => {
            let error_string = e.to_string().replace(token, "***");
            bail!("{}", error_string);
        }
    };

    for asset in data.assets.iter() {
        if asset.name.eq(filename) {
            // this is the correct asset!
            let download = asset
                .url
                .replace("api.github.com", &format!("{token}@api.github.com"));

            let file = File::create(out).context("create so file failed")?;
            let mut buf = BufWriter::new(file);

            download_file_report(&download, &mut buf, |_, _| {}).with_context(|| {
                format!(
                    "Failed while downloading {} to {}",
                    download.replace(token, "{token}").blue(),
                    out.display().file_path_color()
                )
            })?;
            break;
        }
    }

    Ok(out.exists())
}

pub fn clone(mut url: String, branch: Option<&String>, out: &Path) -> Result<bool> {
    check_git()?;
    if let Ok(token_unwrapped) = get_keyring().get_password()
        && let Some(gitidx) = url.find("github.com")
    {
        url.insert_str(gitidx, &format!("{token_unwrapped}@"));
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

                if let Ok(token_unwrapped) = get_keyring().get_password() {
                    error_string = error_string.replace(&token_unwrapped, "***");
                }

                bail!("Exit code {}: {}", e, error_string);
            }
        }
        Err(e) => {
            let mut error_string = e.to_string();

            if let Ok(token_unwrapped) = get_keyring().get_password() {
                error_string = error_string.replace(&token_unwrapped, "***");
            }

            bail!("{}", error_string);
        }
    }

    Ok(out.try_exists()?)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubReleaseAsset {
    pub url: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubReleaseData {
    pub assets: Vec<GithubReleaseAsset>,
}
