use std::{fs::File, path::Path};

use clap::{Args, Subcommand};
use color_eyre::{
    eyre::{bail, eyre, Context},
    Result,
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use qpm_package::models::package::PackageConfig;
use semver::{Version, VersionReq};
use std::io::Write;

use crate::{
    models::{
        android_repo::{AndroidRepositoryManifest, RemotePackage},
        config::get_combine_config,
        package::PackageConfigExtensions,
    },
    resolver::semver::{req_to_range, VersionWrapper},
    terminal::colors::QPMColor,
    utils::android::{
        download_ndk_version, get_android_manifest, get_ndk_str_versions, get_ndk_str_versions_str,
    },
};

use super::Command;

#[derive(Args, Debug, Clone)]
pub struct Ndk {
    #[clap(subcommand)]
    pub op: NdkOperation,
}

#[derive(Subcommand, Debug, Clone)]
pub enum NdkOperation {
    Download(DownloadArgs),
    List,
    Available(AvailableArgs),
    Path(PathArgs),
    Resolve(ResolveArgs),
    Use(UseArgs),
}

#[derive(Args, Debug, Clone)]
pub struct DownloadArgs {
    version: String,
}

#[derive(Args, Debug, Clone)]
pub struct AvailableArgs {
    page: usize,
}
#[derive(Args, Debug, Clone)]
pub struct PathArgs {
    version: String,
}
#[derive(Args, Debug, Clone)]
pub struct UseArgs {
    version: String,
}
#[derive(Args, Debug, Clone)]
pub struct ResolveArgs {}

fn fuzzy_match_ndk<'a>(
    manifest: &'a AndroidRepositoryManifest,
    version: &str,
) -> Result<(String, &'a RemotePackage)> {
    // Find version matching version
    let ndks_str = get_ndk_str_versions_str(manifest);

    let ndk = ndks_str.get(version);
    match ndk {
        Some(ndk) => Ok((version.to_string(), ndk)),
        None => {
            // fuzzy search version using version ranges
            let fuzzy_version_range = req_to_range(VersionReq::parse(version)?);

            // find version closest to specified
            let ndks = get_ndk_str_versions(manifest);
            let ndks_versions = ndks.keys().sorted().rev().collect_vec();
            let matching_version_opt = ndks_versions
                .iter()
                .find(|probe| fuzzy_version_range.contains(&VersionWrapper((**probe).clone())));

            match matching_version_opt {
                Some(matching_version) => Ok((
                    matching_version.to_string(),
                    ndks.get(matching_version).unwrap(),
                )),
                None => bail!("Could not find ndk version {}", version),
            }
        }
    }
}

impl Command for Ndk {
    fn execute(self) -> Result<()> {
        match self.op {
            NdkOperation::Download(d) => {
                let manifest = get_android_manifest()?;

                let (_version, ndk) = fuzzy_match_ndk(&manifest, &d.version)?;
                download_ndk_version(ndk)?
            }
            NdkOperation::Available(a) => {
                let manifest = get_android_manifest()?;
                let amount_per_page = 5;

                let page_offset = a.page;
                let skip = page_offset * amount_per_page;

                println!("Page: {page_offset}");

                get_ndk_str_versions_str(&manifest)
                    .iter()
                    .sorted_by(|a, b| a.0.cmp(b.0))
                    .rev()
                    .skip(skip)
                    .take(5)
                    .for_each(|(v, p)| println!("{} -> {}", v.blue(), p.display_name.purple()))
            }
            NdkOperation::List => {
                let dir = get_combine_config().get_ndk_installed();

                dir.into_iter()
                    .try_collect::<_, Vec<_>, _>()?
                    .into_iter()
                    .filter(|p| p.path().is_dir())
                    .for_each(|p| {
                        println!(
                            "{} -> {}",
                            p.file_name().to_str().unwrap(),
                            p.path().to_str().unwrap()
                        )
                    })
            }
            NdkOperation::Path(p) => {
                let manifest = get_android_manifest()?;

                let (version, _ndk) = fuzzy_match_ndk(&manifest, &p.version)?;

                let dir = get_combine_config()
                    .ndk_download_path
                    .as_ref()
                    .expect("No NDK download path set");

                let ndk_path = dir.join(version);

                if !ndk_path.exists() {
                    bail!("Path {} not found!", ndk_path.display().red());
                }

                println!("{}", ndk_path.display());
            }
            NdkOperation::Resolve(r) => {
                let package = PackageConfig::read(".")?;
                let ndk_requirement = package.workspace.ndk.as_ref();

                let Some(ndk_requirement) = ndk_requirement else {
                    bail!("No NDK requirement set in project")
                };
                // find latest NDK that satisfies requirement
                let ndk_installed_path = get_combine_config()
                    .get_ndk_installed()
                    .into_iter()
                    .flatten()
                    .sorted_by(|a, b| a.file_name().cmp(b.file_name()))
                    .rev() // descending
                    .find(|s| {
                        Version::parse(s.file_name().to_str().unwrap())
                            .is_ok_and(|version| ndk_requirement.matches(&version))
                    })
                    .ok_or_else(|| {
                        eyre!("No NDK version found that satisfies {ndk_requirement}")
                    })?;

                apply_ndk(ndk_installed_path.path())?;
            }
            NdkOperation::Use(u) => {
                let manifest = get_android_manifest()?;
                let (version, ndk) = fuzzy_match_ndk(&manifest, &u.version)?;

                let mut package = PackageConfig::read(".")?;
                let req = format!("^{version}");
                package.workspace.ndk = Some(VersionReq::parse(&req)?);

                let ndk_path = format!(
                    "{}/{version}",
                    get_combine_config()
                        .ndk_download_path
                        .as_ref()
                        .unwrap()
                        .to_str()
                        .unwrap()
                );

                apply_ndk(Path::new(&ndk_path))?;
            }
        }

        Ok(())
    }
}

fn apply_ndk(ndk_installed_path: &Path) -> Result<(), color_eyre::eyre::Error> {
    let mut ndk_file = File::create("./ndkpath.txt").context("Unable to open ndkpath.txt")?;
    writeln!(ndk_file, "{}", ndk_installed_path.to_str().unwrap())?;
    println!("{}", ndk_installed_path.to_str().unwrap());
    Ok(())
}
