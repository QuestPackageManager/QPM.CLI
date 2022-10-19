use color_eyre::{eyre::{Context, bail}, Result};
use reqwest::StatusCode;
use semver::Version;
use std::collections::HashMap;

use serde::Deserialize;

use crate::{
    models::{dependency::{SharedPackageConfig}, backend::PackageVersion, package::PackageConfig},
    network::agent::get_agent,
};

use super::Repository;

const API_URL: &str = "https://qpackages.com";

#[derive(Default)]
pub struct QPMRepository {
    packages_cache: HashMap<String, HashMap<Version, SharedPackageConfig>>,
}

impl QPMRepository {

    fn run_request<T: for<'a> Deserialize<'a>>(path: &str) -> Result<Option<T>> {
        let url = format!("{}/{}", API_URL, path);

        let response = get_agent()
            .get(&url)
            .send()
            .context("Unable to make request to qpackages.com")?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let result: T = response.json().expect("Into json failed");

        Ok(Some(result))
    }

    /// Requests the appriopriate package info from qpackage.com
    pub fn get_versions(id: &str) -> Result<Option<Vec<PackageVersion>>> {
        Self::run_request(&format!("{}?limit=0", id))
    }

    pub fn get_shared_package(id: &str, ver: &Version) -> Result<Option<SharedPackageConfig>> {
        Self::run_request(&format!("{}/{}", id, ver))
    }

    pub fn get_packages() -> Result<Vec<String>> {
        Ok(Self::run_request("")?.unwrap())
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
            bail!(
                "Could not publish to {}: Unauthorized! Did you provide the correct key?",
                API_URL
            );
        }
        resp.error_for_status()?;
        Ok(())
    }
}

impl Repository for QPMRepository {
    fn get_package_names(&self) -> Result<Vec<String>> {
        Self::get_packages()
    }

    fn get_package_versions(&self, id: &str) -> Result<Option<Vec<PackageVersion>>> {
        let cache = self.packages_cache.get(id).map(|f| {
            f.keys()
                .map(|v| PackageVersion {
                    id: id.to_string(),
                    version: v.clone(),
                })
                .collect::<Vec<_>>()
        });

        if let Some(c) = cache {
            return Ok(Some(c));
        }

        Self::get_versions(id)
    }

    fn get_package(&self, id: &str, version: &Version) -> Result<Option<SharedPackageConfig>> {
        let cache = self.packages_cache.get(id).and_then(|f| f.get(&version));

        if let Some(c) = cache {
            return Ok(Some(c.clone()));
        }

        Self::get_shared_package(id, &version)
    }

    fn add_to_cache(&mut self, config: SharedPackageConfig, permanent: bool) -> Result<()> {
        self.packages_cache
            .entry(config.config.info.id.clone())
            .or_default()
            .entry(config.config.info.version.clone())
            .insert_entry(config);
        Ok(())
    }
}
