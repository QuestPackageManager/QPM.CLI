use std::fs::File;

use clap::{Args, Subcommand};
use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use qpm_package::models::package::PackageConfig;
use semver::{Version, VersionReq};
use std::io::Write;

use crate::{
    models::{config::get_combine_config, package::PackageConfigExtensions},
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
pub struct UseArgs {
    version: String,
}

impl Command for Ndk {
    fn execute(self) -> Result<()> {
        match self.op {
            NdkOperation::Use(u) => do_use(u)?,
            NdkOperation::Download(d) => do_download(d)?,
            NdkOperation::Available(a) => {
                do_available(a)?;
            }
            NdkOperation::List => do_list()?,
        }

        Ok(())
    }
}

fn do_use(u: UseArgs) -> Result<()> {
    let ndk_installed = get_combine_config()
        .get_ndk_installed()
        .into_iter()
        .flatten()
        .find(|s| s.file_name().to_string_lossy() == u.version);

    if ndk_installed.is_none() {
        bail!("Could not find NDK version {}", u.version);
    }
    let ndk_path = ndk_installed
        .map(|p| p.path().as_os_str().to_string_lossy().to_string())
        .unwrap();

    let package = PackageConfig::read(".")?;
    let ndk_requirement = package.workspace.ndk.as_ref();

    if let Some(ndk_requirement) = ndk_requirement {
        let version = Version::parse(&u.version).with_context(|| format!("Could not parse version {} while package requires to satisfy range {ndk_requirement}", u.version))?;

        let satisfies_range = ndk_requirement.matches(&version);

        if !satisfies_range {
            bail!("Version {version} does not satisfy NDK requirement {satisfies_range}");
        }
    }

    let mut ndk_file = File::create("./ndkpath.txt").context("Unable to open ndkpath.txt")?;

    writeln!(ndk_file, "{}", ndk_path)?;

    println!("Using NDK at {}", ndk_path.file_path_color());

    Ok(())
}

fn do_list() -> Result<()> {
    get_combine_config()
        .get_ndk_installed()
        .into_iter()
        .try_collect::<_, Vec<_>, _>()?
        .into_iter()
        .filter(|p| p.path().is_dir())
        .for_each(|p| {
            println!(
                "{} -> {}",
                p.file_name().to_str().unwrap(),
                p.path().to_str().unwrap()
            )
        });
    Ok(())
}

fn do_available(a: AvailableArgs) -> Result<(), color_eyre::eyre::Error> {
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
        .for_each(|(v, p)| println!("{} -> {}", v.blue(), p.display_name.purple()));
    Ok(())
}

fn do_download(d: DownloadArgs) -> Result<()> {
    let manifest = get_android_manifest()?;
    let ndks_str = get_ndk_str_versions_str(&manifest);
    let ndk = ndks_str.get(d.version.as_str());
    match ndk {
        Some(ndk) => download_ndk_version(ndk)?,
        None => {
            // fuzzy search version using version ranges
            let fuzzy_version_range = req_to_range(VersionReq::parse(&d.version)?);

            // find version closest to specified
            let ndks = get_ndk_str_versions(&manifest);
            let ndks_versions = ndks.keys().sorted().rev().collect_vec();
            let matching_version_opt = ndks_versions
                .iter()
                .find(|probe| fuzzy_version_range.contains(&VersionWrapper((**probe).clone())));

            match matching_version_opt {
                Some(matching_version) => {
                    download_ndk_version(ndks.get(matching_version).unwrap())?
                }
                None => bail!("Could not find ndk version {}", d.version),
            }
        }
    };

    Ok(())
}
