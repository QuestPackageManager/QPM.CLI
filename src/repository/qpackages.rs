use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use reqwest::StatusCode;
use semver::Version;
use std::{
    cell::UnsafeCell,
    collections::HashMap,
    fs::{self, File},
    io::{Cursor, Write},
    path::Path,
};
use zip::ZipArchive;

use serde::Deserialize;

use qpm_package::models::{
    backend::PackageVersion, dependency::SharedPackageConfig, package::PackageConfig,
};

use crate::{
    models::{
        config::get_combine_config, package::PackageConfigExtensions,
        package_metadata::PackageMetadataExtensions,
    },
    network::agent::{download_file_report, get_agent},
    terminal::colors::QPMColor,
    utils::git,
};

use super::Repository;

const API_URL: &str = "https://qpackages.com";

#[derive(Default)]
pub struct QPMRepository {
    // interior mutability
    packages_cache: UnsafeCell<HashMap<String, HashMap<Version, SharedPackageConfig>>>,
    versions_cache: UnsafeCell<HashMap<String, Vec<PackageVersion>>>,
}

impl QPMRepository {
    fn run_request<T>(path: &str) -> Result<Option<T>>
    where
        T: for<'a> Deserialize<'a>,
    {
        let url = format!("{API_URL}/{path}");

        let response = get_agent()
            .get(url)
            .send()
            .context("Unable to make request to qpackages.com")?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let result: T = response.json().context("Into json failed")?;

        Ok(Some(result))
    }

    /// Requests the appriopriate package info from qpackage.com
    pub fn get_versions(id: &str) -> Result<Option<Vec<PackageVersion>>> {
        Self::run_request(&format!("{id}?limit=0"))
    }

    pub fn get_shared_package(id: &str, ver: &Version) -> Result<Option<SharedPackageConfig>> {
        Self::run_request(&format!("{id}/{ver}"))
    }

    pub fn get_packages() -> Result<Vec<String>> {
        Ok(Self::run_request("")?.unwrap())
    }

    pub fn publish_package(package: &SharedPackageConfig, auth: &str) -> Result<()> {
        let url = format!(
            "{}/{}/{}",
            API_URL, &package.config.info.id, &package.config.info.version
        );

        let resp = get_agent()
            .post(url)
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

    fn download_package(&self, config: &PackageConfig) -> Result<()> {
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

        println!(
            "Checking cache for dependency {} {}",
            config.info.id.bright_red(),
            config.info.version.bright_green()
        );
        let user_config = get_combine_config();
        let base_path = user_config
            .cache
            .as_ref()
            .unwrap()
            .join(&config.info.id)
            .join(config.info.version.to_string());

        let src_path = base_path.join("src");
        let lib_path = base_path.join("lib");
        let tmp_path = base_path.join("tmp");

        if src_path.join("qpm.shared.json").exists() {
            // ensure is valid
            SharedPackageConfig::read(src_path).with_context(|| {
                format!(
                    "Failed to get config {}:{} in cache",
                    config.info.id, config.info.version
                )
            })?;
            return Ok(());
        }

        let so_path = lib_path.join(config.info.get_so_name());
        let debug_so_path = lib_path.join(format!("debug_{}", config.info.get_so_name()));

        // Downloads the repo / zip file into src folder w/ subfolder taken into account
        if !src_path.exists() {
            // if the tmp path exists, but src doesn't, that's a failed cache, delete it and try again!
            if tmp_path.exists() {
                fs::remove_dir_all(&tmp_path).with_context(|| format!("Failed to remove existing tmp folder {tmp_path:?}"))?;
            }

            // src did not exist, this means that we need to download the repo/zip file from packageconfig.info.url
            fs::create_dir_all(&base_path)
                .with_context(|| format!("Failed to create lib path {base_path:?}"))?;
            let url = config.info.url.as_ref().unwrap();
            if url.contains("github.com") {
                // github url!
                git::clone(
                    url.clone(),
                    config.info.additional_data.branch_name.as_ref(),
                    &tmp_path,
                )?;
            } else {
                // not a github url, assume it's a zip
                let response = get_agent().get(url).send()?;

                let buffer = Cursor::new(response.bytes()?);
                // Extract to tmp folder
                ZipArchive::new(buffer)?.extract(&tmp_path)?;
            }
            // the only way the above if else would break is if someone put a link to a zip file on github in the url slot
            // if you are reading this and think of doing that so I have to fix this, fuck you

            let sub_package_path = match &config.info.additional_data.sub_folder {
                Some(sub_folder) => {
                    // the package exists in a subfolder of the downloaded thing, just move the subfolder to src
                    tmp_path.join(sub_folder)
                }
                _ => {
                    // the downloaded thing IS the package, just rename the folder to src
                    tmp_path.clone()
                }
            };

            if sub_package_path.exists() {
                // only log this on debug builds
                #[cfg(debug_assertions)]
                println!(
                    "from: {}\nto: {}",
                    sub_package_path.display().bright_yellow(),
                    src_path.display().bright_yellow()
                );

                if src_path.exists() {
                    let mut line = String::new();
                    println!(
                        "Confirm deletion of folder {}: (y/N)",
                        src_path.display().bright_yellow()
                    );
                    std::io::stdin().read_line(&mut line)?;
                    if line.starts_with('y') || line.starts_with('Y') {
                        fs::remove_dir_all(&src_path)
                            .context("Failed to remove existing src folder")?;
                    }
                }
                // HACK: renaming seems to work, idk if it works for actual subfolders?
                fs::rename(&sub_package_path, &src_path).context("Failed to move folder")?;
            } else {
                bail!("Failed to restore folder for this dependency\nif you have a token configured check if it's still valid\nIf it is, check if you can manually reach the repo");
            }

            // clear up tmp folder if it still exists
            if tmp_path.exists() {
                std::fs::remove_dir_all(tmp_path).context("Failed to remove tmp folder")?;
            }
            let package_path = src_path;
            let downloaded_package = SharedPackageConfig::read(package_path);

            match downloaded_package {
                Ok(downloaded_package) =>
                // check if downloaded config is the same version as expected, if not, panic
                {
                    if downloaded_package.config.info.version != config.info.version {
                        bail!(
                            "Downloaded package ({}) version ({}) does not match expected version ({})!",
                            config.info.id.bright_red(),
                            downloaded_package
                                .config
                                .info
                                .version
                                .to_string()
                                .bright_green(),
                            config.info.version.to_string().bright_green(),
                        )
                    }
                }

                Err(e) => println!(
                    "Unable to validate shared package of {}:{} due to: \"{}\", continuing",
                    config.info.name.dependency_id_color(),
                    config.info.version.dependency_version_color(),
                    e.red()
                ),
            }
        }

        if !lib_path.exists() {
            fs::create_dir_all(&lib_path).context("Failed to create lib path")?;
            // libs didn't exist or the release object didn't exist, we need to download from packageconfig.info.additional_data.so_link and packageconfig.info.additional_data.debug_so_link
            let download_binary = |path: &Path, url_opt: Option<&String>| -> Result<_> {
                if !path.exists() || File::open(path).is_err() {
                    if let Some(url) = url_opt {
                        // so_link existed, download
                        if url.contains("github.com") {
                            // github url!
                            git::get_release(url, path)?;
                        } else {
                            let bytes = download_file_report(url, |_, _| {})?;

                            let mut file = File::create(path)?;

                            file.write_all(&bytes)
                                .context("Failed to write out downloaded bytes")?;
                        }
                    }
                }
                Ok(())
            };

            download_binary(&so_path, config.info.additional_data.so_link.as_ref())?;
            download_binary(
                &debug_so_path,
                config.info.additional_data.debug_so_link.as_ref(),
            )?;
        }
        Ok(())
    }
}

impl Repository for QPMRepository {
    fn get_package_names(&self) -> Result<Vec<String>> {
        Self::get_packages()
    }

    fn get_package_versions(&self, id: &str) -> Result<Option<Vec<PackageVersion>>> {
        let cache = self.versions_cache.get_safe().get(id);

        if let Some(c) = cache {
            return Ok(Some(c.clone()));
        }

        let versions = Self::get_versions(id)?.map(|versions| {
            versions
                .into_iter()
                .sorted_by(|a, b| a.version.cmp(&b.version))
                .rev()
                .collect_vec()
        });

        if let Some(versions) = &versions {
            self.versions_cache
                .get_mut_safe()
                .entry(id.to_string())
                .insert_entry(versions.clone());
        }

        Ok(versions)
    }

    fn get_package(&self, id: &str, version: &Version) -> Result<Option<SharedPackageConfig>> {
        let cache = self
            .packages_cache
            .get_safe()
            .get(id)
            .and_then(|f| f.get(version));

        if let Some(c) = cache {
            return Ok(Some(c.clone()));
        }

        let config = Self::get_shared_package(id, version)?;

        if let Some(config) = &config {
            self.packages_cache
                .get_mut_safe()
                .entry(config.config.info.id.clone())
                .or_default()
                .entry(config.config.info.version.clone())
                .insert_entry(config.clone());
        }

        Ok(config)
    }

    fn add_to_db_cache(&mut self, _config: SharedPackageConfig, _permanent: bool) -> Result<()> {
        Ok(())
    }

    fn download_to_cache(&mut self, config: &PackageConfig) -> Result<bool> {
        self.download_package(config)?;

        Ok(true)
    }

    fn write_repo(&self) -> Result<()> {
        Ok(())
    }

    fn is_online(&self) -> bool {
        true
    }
}
trait UnsafeCellExt<T>: Sized {
    fn get_safe(&self) -> &T;

    #[allow(clippy::mut_from_ref)]
    fn get_mut_safe(&self) -> &mut T;
}

impl<T> UnsafeCellExt<T> for UnsafeCell<T> {
    fn get_safe(&self) -> &T {
        unsafe { &*self.get() }
    }

    fn get_mut_safe(&self) -> &mut T {
        unsafe { &mut *self.get() }
    }
}
