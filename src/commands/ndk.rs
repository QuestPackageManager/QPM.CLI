use std::path::PathBuf;

use clap::{Args, Subcommand};
use color_eyre::{Result, eyre::bail};
use itertools::Itertools;
use owo_colors::OwoColorize;
use qpm_package::models::package::PackageConfig;
use semver::VersionReq;

use crate::{
    models::{config::get_combine_config, package::PackageConfigExtensions},
    services::android::{download_ndk_version, get_android_manifest, get_ndk_str_versions_str},
    services::ndk as ndk_service,
};

use super::Command;

#[derive(Args, Debug, Clone)]
pub struct Ndk {
    #[clap(subcommand)]
    pub op: NdkOperation,

    /// If true, does not print progress
    #[clap(long, short, global = true, default_value = "false")]
    pub quiet: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum NdkOperation {
    Download(DownloadArgs),
    List,
    Available(AvailableArgs),
    Path(PathArgs),
    ///  Apply the current NDK requirements (installed only, highest version that is valid). Download if necessary
    Resolve(ResolveArgs),
    /// Set the current NDK requirements (highest installed NDK version, allow non-installed versions if desired)
    Pin(PinArgs),
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
pub struct PinArgs {
    /// NDK Version that should be required
    version: String,

    /// Use strict = for version constraint
    #[clap(long, default_value = "false")]
    strict: bool,

    /// If true, allows versions that are not installed
    #[clap(long, default_value = "false")]
    online: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ResolveArgs {
    /// Download package if necessary
    #[clap(short, long, default_value = "false")]
    pub download: bool,

    // Ignore missing package
    #[clap(short, long, default_value = "false")]
    pub ignore_missing: bool,
}

impl Command for Ndk {
    fn execute(self) -> Result<()> {
        match self.op {
            NdkOperation::Download(d) => {
                let manifest = get_android_manifest()?;

                let (_version, ndk) = ndk_service::fuzzy_match_ndk(&manifest, &d.version)?;
                let ndk_download_path = get_combine_config()
                    .ndk_download_path
                    .as_ref()
                    .expect("No NDK download path set");
                let path = download_ndk_version(ndk, !self.quiet, ndk_download_path)?;
                println!("{}", path.display());
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

                let (version, _ndk) = ndk_service::fuzzy_match_ndk(&manifest, &p.version)?;

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
            NdkOperation::Resolve(r) => do_resolve(r, self.quiet)?,
            NdkOperation::Pin(u) => do_pin(u)?,
        }

        Ok(())
    }
}

fn do_pin(u: PinArgs) -> Result<()> {
    let installed_ndks = get_combine_config()
        .get_ndk_installed()
        .into_iter()
        .flatten()
        .map(|e| e.into_path())
        .collect_vec();

    let version = ndk_service::resolve_pin_version(&u.version, u.online, &installed_ndks)?;

    let mut package = PackageConfig::read(".")?;

    let req = match u.strict {
        true => format!("={version}"),
        false => format!("^{version}"),
    };
    package.workspace.ndk = Some(VersionReq::parse(&req)?);
    package.write(".")?;

    let ndk_path: PathBuf = get_combine_config()
        .ndk_download_path
        .as_ref()
        .unwrap()
        .join(version);

    ndk_service::apply_ndk(&ndk_path)?;
    println!("{}", ndk_path.display());
    Ok(())
}

fn do_resolve(r: ResolveArgs, quiet: bool) -> Result<()> {
    let package = PackageConfig::exists(".")
        .then(|| PackageConfig::read("."))
        .transpose()?;

    let Some(package) = package else {
        if r.ignore_missing {
            return Ok(());
        }
        bail!("No package found in current directory")
    };

    let ndk_download_path = get_combine_config()
        .ndk_download_path
        .clone()
        .expect("No NDK download path set");
    let ndk_installed_path =
        ndk_service::resolve_ndk_for_package(&package, r.download, quiet, &ndk_download_path)?;

    ndk_service::apply_ndk(&ndk_installed_path)?;
    println!("{}", ndk_installed_path.display());
    Ok(())
}
