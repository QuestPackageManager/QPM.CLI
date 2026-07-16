use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, OptionExt, bail},
};
use itertools::Itertools;
use semver::Version;

use serde::Deserialize;

use qpm_package::models::{
    package::{DependencyId, PackageConfig},
    qpackages::{QPackagesPackage, QPackagesVersion},
};
use ureq::http::StatusCode;

use crate::{
    models::{package_files::PackageIdPath, qpackages::QPackageExtensions},
    repository::file::FileRepository,
    services::network::get_agent,
    terminal::colors::QPMColor,
};

use super::{Artifact, Repository};

const API_URL: &str = "https://new.qpackages.com";

#[derive(Default)]
pub struct QPMRepository {}

impl QPMRepository {
    fn run_request<T>(path: &str) -> Result<Option<T>>
    where
        T: for<'a> Deserialize<'a>,
    {
        let url = format!("{API_URL}/{path}");

        let response = get_agent()
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
    pub fn get_versions(id: &DependencyId) -> Result<Option<Vec<QPackagesVersion>>> {
        Self::run_request(&format!("{id}?limit=0"))
            .with_context(|| format!("Getting list of versions for {}", id.dependency_id_color()))
    }

    pub fn get_qpackage(id: &DependencyId, ver: &Version) -> Result<Option<QPackagesPackage>> {
        Self::run_request(&format!("{id}/{ver}")).with_context(|| {
            format!(
                "Getting shared package config {}:{}",
                id.dependency_id_color(),
                ver.version_id_color()
            )
        })
    }

    pub fn get_packages() -> Result<Vec<DependencyId>> {
        let vec = Self::run_request("")
            .with_context(|| format!("{API_URL} packages list failed"))?
            .ok_or_eyre("No packages found?")?;
        Ok(vec)
    }

    pub fn publish_package(qpackage: &QPackagesPackage, auth: &str) -> Result<()> {
        let url = format!(
            "{}/{}/{}",
            API_URL, &qpackage.config.id, &qpackage.config.version
        );

        let resp = get_agent()
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

    /// Downloads the package and caches it in the user config cache path
    /// Note this does not depend necessarily on it being hosted at `API_URL`, it can be any valid QPkg
    fn download_package(&self, qpackage_config: &QPackagesPackage) -> Result<()> {
        // If a `qpackages.json` marker already exists for this id:version, it's already cached
        // and there's nothing to do. Otherwise download the QPKG from `qpkg_url`, verify its
        // checksum, extract it into the cache, and write the marker.
        let config = &qpackage_config.config;

        println!(
            "Checking cache for dependency {} {}",
            config.id.dependency_id_color(),
            config.version.version_id_color()
        );
        let package_path = PackageIdPath::new(config.id.clone()).version(config.version.clone());

        let mut file_repo = FileRepository::read_global_cache()?;
        let base_path = package_path.base_path(file_repo.root());

        let qpackages_cached = QPackagesPackage::read(&base_path);
        if let Ok(qpackages_cached) = qpackages_cached {
            if qpackages_cached != *qpackage_config {
                eprintln!(
                    "Cached QPackages {}:{} does not match the requested {}:{}",
                    qpackages_cached.config.id.dependency_id_color(),
                    qpackages_cached.config.version.version_id_color(),
                    config.id.dependency_id_color(),
                    config.version.version_id_color()
                );
            }
            // already cached, no need to download again
            return Ok(());
        }

        let qpkg_url = &qpackage_config.qpkg_url;

        let package = file_repo
            .install_qpkg_from_url(
                qpkg_url,
                qpackage_config.qpkg_checksum.as_deref(),
                false,
                None,
            )
            .with_context(|| {
                format!(
                    "QPackages QPKG installation from {}:{}",
                    qpackage_config.config.id, qpackage_config.config.version
                )
            })?;

        // assert that the package is the same as the one we downloaded
        if package.config != qpackage_config.config {
            bail!(
                "Package config mismatch. Got {}:{}: expected {:?}, got {:?} (the changes might not be id/version, but the config itself)",
                package.config.id.dependency_id_color(),
                package.config.version.version_id_color(),
                qpackage_config.config,
                package.config
            );
        }

        qpackage_config.write(&base_path).with_context(|| {
            format!(
                "Failed to write QPackages.json to {}",
                base_path.display().file_path_color()
            )
        })?;

        Ok(())
    }
}

impl Repository for QPMRepository {
    fn get_package_names(&self) -> Result<Vec<DependencyId>> {
        Self::get_packages()
    }

    /// Sorted descending order
    fn get_package_versions(&self, id: &DependencyId) -> Result<Option<Vec<Version>>> {
        let versions = Self::get_versions(id)?.map(|versions| {
            versions
                .into_iter()
                .map(|v| v.version)
                .sorted_by(|a, b| a.cmp(b))
                .rev()
                .collect_vec()
        });

        Ok(versions)
    }

    fn get_package(&self, id: &DependencyId, version: &Version) -> Result<Option<Artifact>> {
        let config = Self::get_qpackage(id, version)?;

        Ok(config.map(|qpackage| Artifact {
            config: qpackage.config,
            qpkg_checksum: qpackage.qpkg_checksum,
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

    fn download_to_cache(&mut self, config: &PackageConfig) -> Result<bool> {
        let qpackage =
            QPMRepository::get_qpackage(&config.id, &config.version)?.with_context(|| {
                format!(
                    "Failed to get QPackage for {}:{}",
                    config.id.dependency_id_color(),
                    config.version.version_id_color()
                )
            })?;

        self.download_package(&qpackage).with_context(|| {
            format!(
                "QPackages {}:{}",
                config.id.dependency_id_color(),
                config.version.version_id_color()
            )
        })?;

        Ok(true)
    }

    fn write_repo(&self) -> Result<()> {
        Ok(())
    }

    fn is_online(&self) -> bool {
        true
    }
}
