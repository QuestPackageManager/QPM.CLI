use color_eyre::{
    Result,
    eyre::{Context, OptionExt, bail},
};
use itertools::Itertools;
use semver::Version;

use serde::Deserialize;

use qpm_package::models::{
    package::{DependencyId, PackageConfig},
    qpackages::{QPackagesPackage, QPackagesVersion},
};
use ureq::http::StatusCode;

use crate::terminal::colors::QPMColor;

use super::{Artifact, Repository, Versions};

const API_URL: &str = "https://new.qpackages.com";

/// The qpackages index: package lookups and publishing. Downloading a looked-up package's
/// `qpkg_url` is done by the local cache ([`FileRepository`](super::file::FileRepository)),
/// since URLs don't only come from qpackages.
pub struct QPMRepository {
    agent: ureq::Agent,
}

impl QPMRepository {
    pub fn new(agent: ureq::Agent) -> Self {
        Self { agent }
    }

    fn run_request<T>(&self, path: &str) -> Result<Option<T>>
    where
        T: for<'a> Deserialize<'a>,
    {
        let url = format!("{API_URL}/{path}");

        let response = self
            .agent
            .get(&url)
            .call()
            .with_context(|| format!("Unable to make request to {API_URL}: {url}"))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        // ureq::Response doesn't provide `error_for_status_ref`; ensure we got a 2xx status
        if !response.status().is_success() {
            bail!("Request to {url} failed with status {}", response.status());
        }

        let result: T = response
            .into_body()
            .read_json()
            .with_context(|| format!("Into json failed for http request for {url}"))?;

        Ok(Some(result))
    }

    /// Requests the appriopriate package info from qpackage.com
    pub fn get_versions(&self, id: &DependencyId) -> Result<Option<Vec<QPackagesVersion>>> {
        self.run_request(&format!("{id}?limit=0"))
            .with_context(|| format!("Getting list of versions for {}", id.dependency_id_color()))
    }

    pub fn get_qpackage(
        &self,
        id: &DependencyId,
        ver: &Version,
    ) -> Result<Option<QPackagesPackage>> {
        self.run_request(&format!("{id}/{ver}")).with_context(|| {
            format!(
                "Getting shared package config {}:{}",
                id.dependency_id_color(),
                ver.version_id_color()
            )
        })
    }

    pub fn get_packages(&self) -> Result<Vec<DependencyId>> {
        let vec = self
            .run_request("")
            .with_context(|| format!("{API_URL} packages list failed"))?
            .ok_or_eyre("No packages found?")?;
        Ok(vec)
    }

    pub fn publish_package(&self, qpackage: &QPackagesPackage, auth: &str) -> Result<()> {
        let url = format!(
            "{}/{}/{}",
            API_URL, qpackage.config.id, qpackage.config.version
        );

        let resp = self
            .agent
            .post(&url)
            .header("Authorization", auth)
            .send_json(qpackage)
            .with_context(|| format!("Failed to publish to {url}"))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            bail!(
                "Could not publish to {}: Unauthorized! Did you provide the correct key?",
                API_URL
            );
        }
        if !resp.status().is_success() {
            bail!("Could not publish to {}: HTTP {}", API_URL, resp.status());
        }

        Ok(())
    }
}

impl Repository for QPMRepository {
    fn get_package_names(&self) -> Result<Vec<DependencyId>> {
        self.get_packages()
    }

    /// Sorted descending order
    fn get_package_versions(&self, id: &DependencyId) -> Result<Option<Versions>> {
        let versions = self.get_versions(id)?.map(|versions| {
            versions
                .into_iter()
                .map(|v| v.version)
                .sorted_by(|a, b| b.cmp(a)) // highest first
                .collect()
        });

        Ok(versions)
    }

    fn get_package(&self, id: &DependencyId, version: &Version) -> Result<Option<Artifact>> {
        let qpackage = self.get_qpackage(id, version)?;

        Ok(qpackage.map(|qpackage| Artifact {
            config: qpackage.config,
            qpkg_checksum: qpackage.qpkg_checksum,
            qpkg_url: Some(qpackage.qpkg_url),
        }))
    }

    fn add_to_db_cache(
        &mut self,
        _config: PackageConfig,
        _qpkg_checksum: Option<String>,
        _permanent: bool,
    ) -> Result<()> {
        Ok(())
    }

    /// Never installs anything itself; see [`QPMRepository`]
    fn download_to_cache(&mut self, _agent: &ureq::Agent, _artifact: &Artifact) -> Result<bool> {
        Ok(false)
    }

    fn write_repo(&self) -> Result<()> {
        Ok(())
    }

    fn is_online(&self) -> bool {
        true
    }
}
