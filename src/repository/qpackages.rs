use color_eyre::{eyre::Context, Result};
use reqwest::StatusCode;
use semver::Version;

use serde::Deserialize;

use crate::{
    models::package::{PackageVersion, SharedPackageConfig},
    network::agent::get_agent,
};

const API_URL: &str = "https://qpackages.com";

fn run_request<T: for<'a> Deserialize<'a>>(path: &str) -> Result<T> {
    let url = format!("{}/{}", API_URL, path);

    let response = get_agent()
        .get(&url)
        .send()
        .context("Unable to make request to qpackages.com")?;

    let result: T = response.json().expect("Into json failed");

    Ok(result)
}

/// Requests the appriopriate package info from qpackage.com
pub fn get_versions(id: &str) -> Result<Vec<PackageVersion>> {
    run_request(&format!("{}?limit=0", id))
}

pub fn get_shared_package(id: &str, ver: &Version) -> Result<SharedPackageConfig> {
    run_request(&format!("{}/{}", id, ver))
}

pub fn get_packages() -> Result<Vec<String>> {
    run_request("")
}

pub fn publish_package(package: &SharedPackageConfig, auth: &str) -> Result<()> {
    // TODO:
    // let url = format!(
    //     "{}/{}/{}",
    //     API_URL, &package.config.info.id, &package.config.info.version
    // );
    let url = format!("{}/", API_URL);

    let resp = get_agent()
        .post(&url)
        .header("Authorization", auth)
        .json(&package)
        .send()?;

    if resp.status() == StatusCode::UNAUTHORIZED {
        panic!(
            "Could not publish to {}: Unauthorized! Did you provide the correct key?",
            API_URL
        );
    }
    resp.error_for_status()?;
    Ok(())
}
