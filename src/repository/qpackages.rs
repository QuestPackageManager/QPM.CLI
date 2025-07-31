use bytes::{BufMut, BytesMut};
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, OptionExt, bail},
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use reqwest::StatusCode;
use semver::Version;
use sha2::{Digest, Sha256};
use std::io::Read;

use serde::Deserialize;

use qpm_package::models::{
    package::{DependencyId, PackageConfig},
    qpackages::QPackagesPackage,
    shared_package::SharedPackageConfig,
};

use crate::{
    models::{
        package::PackageConfigExtensions, package_files::PackageIdPath,
        qpackages::QPackageExtensions, qpkg::QPkgExtensions,
    },
    network::agent::{download_file_report, get_agent},
    repository::local::FileRepository,
    terminal::colors::QPMColor,
};

use super::Repository;

const API_URL: &str = "https://2.qpackages.com";

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
            .send()
            .with_context(|| format!("Unable to make request to qpackages.com {url}"))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        response.error_for_status_ref()?;

        let result: T = response
            .json()
            .with_context(|| format!("Into json failed for http request for {url}"))?;

        Ok(Some(result))
    }

    /// Requests the appriopriate package info from qpackage.com
    pub fn get_versions(id: &DependencyId) -> Result<Option<Vec<Version>>> {
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
            .context("qpackages.com packages list failed")?
            .ok_or_eyre("No packages found?")?;
        Ok(vec)
    }

    pub fn publish_package(package: &SharedPackageConfig, auth: &str) -> Result<()> {
        let url = format!(
            "{}/{}/{}",
            API_URL, &package.config.id, &package.config.version
        );

        let resp = get_agent()
            .post(&url)
            .header("Authorization", auth)
            .json(&package)
            .send()
            .with_context(|| format!("Failed to publish to {url}"))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            bail!(
                "Could not publish to {}: Unauthorized! Did you provide the correct key?",
                API_URL
            );
        }
        resp.error_for_status()?;
        Ok(())
    }

    /// Downloads the package and caches it in the user config cache path
    /// Note this does not depend necessarily on it being on qpackages.com, it can be any valid QPkg
    fn download_package(&self, qpackage_config: &QPackagesPackage) -> Result<()> {
        // Check if already cached
        // if true, don't download repo / header files
        // else cache to tmp folder in package id folder @ cache path
        //          git repo -> git clone w/ or without github token
        //          not git repo (no github.com) -> assume it's a zip
        //          !! HANDLE SUBFOLDER FROM TMP, OR IF NO SUBFOLDER JUST RENAME TMP TO SRC !!
        //          -- now we have the header files --
        // Check if .so files are downloaded, if not:
        // Download release .so and possibly debug .so to libs folder, if from github use token if available
        // Now it should be cached!

        let config = &qpackage_config.config;

        println!(
            "Checking cache for dependency {} {}",
            config.id.dependency_id_color(),
            config.version.version_id_color()
        );
        let package_path = PackageIdPath::new(config.id.clone()).version(config.version.clone());

        let base_path = package_path.base_path();

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
        let mut bytes = BytesMut::new().writer();

        println!("Downloading {}", qpkg_url.file_path_color());
        download_file_report(qpkg_url, &mut bytes, |_, _| {})
            .with_context(|| format!("Failed while downloading {}", qpkg_url.blue()))?;

        let cursor = std::io::Cursor::new(bytes.get_ref());

        // Ensure checksum matches
        if let Some(checksum) = &qpackage_config.qpkg_checksum {
            let result = Sha256::digest(cursor.get_ref());
            let hash_hex = hex::encode(result);

            if !hash_hex.eq_ignore_ascii_case(checksum) {
                bail!(
                    "Checksum mismatch for {}: expected {}, got {}",
                    qpkg_url.blue(),
                    checksum,
                    hash_hex
                );
            }
        }

        FileRepository::install_qpkg(cursor, false).with_context(|| {
            format!(
                "QPackages QPKG installation from {}:{}",
                qpackage_config.config.id, qpackage_config.config.version
            )
        })?;

        let package = PackageConfig::read(&base_path)?;
        // assert that the package is the same as the one we downloaded
        if package != qpackage_config.config {
            bail!(
                "Package config mismatch. Got {}:{}: expected {:?}, got {:?} (the changes might not be id/version, but the config itself)",
                package.id.dependency_id_color(),
                package.version.version_id_color(),
                qpackage_config.config,
                package
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
                .sorted_by(|a, b| a.cmp(b))
                .rev()
                .collect_vec()
        });

        Ok(versions)
    }

    fn get_package(&self, id: &DependencyId, version: &Version) -> Result<Option<PackageConfig>> {
        let config = Self::get_qpackage(id, version)?;

        Ok(config.map(|qpackage| qpackage.config))
    }

    fn add_to_db_cache(&mut self, _config: PackageConfig, _permanent: bool) -> Result<()> {
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
