use std::{
    fs::File,
    path::{Path, PathBuf},
};

use clap::{Args, Subcommand};
use color_eyre::{
    eyre::{bail, eyre, Context},
    Result, Section,
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use qpm_package::models::package::PackageConfig;
use semver::{Version, VersionReq};
use std::io::Write;

use crate::{
    commands::ndk, models::{
        android_repo::{AndroidRepositoryManifest, RemotePackage},
        config::get_combine_config,
        package::PackageConfigExtensions,
    }, resolver::semver::{req_to_range, VersionWrapper}, terminal::colors::QPMColor, utils::android::{
        download_ndk_version, get_android_manifest, get_ndk_str_versions, get_ndk_str_versions_str,
    }
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
    ///  Apply the current NDK requirements (installed only, highest version that is valid). Download if necessary
    Resolve(ResolveArgs),
    /// Set the current NDK requirements (highest installed NDK version, allow non-installed versions if desired)
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
    /// NDK Version that should be required
    version: String,

    /// Use strict = for version constraint
    #[clap(long, default_value = "false")]
    strict: bool,

    #[clap(long, default_value = "true")]
    installed_only: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ResolveArgs {
    /// Download package if necessary
    #[clap(short, long, default_value = "false")]
    download: bool,
}

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
            let fuzzy_version_range = VersionReq::parse(version)?;

            range_match_ndk(manifest, &fuzzy_version_range)
        }
    }
}

fn range_match_ndk<'a>(
    manifest: &'a AndroidRepositoryManifest,
    fuzzy_version_range: &VersionReq,
) -> Result<(String, &'a RemotePackage)> {
    // find version closest to specified
    let ndks = get_ndk_str_versions(manifest);
    let ndks_versions = ndks.keys().sorted().rev().collect_vec();
    let matching_version_opt = ndks_versions
        .iter()
        .find(|probe| fuzzy_version_range.matches(probe));

    match matching_version_opt {
        Some(matching_version) => Ok((
            matching_version.to_string(),
            ndks.get(matching_version).unwrap(),
        )),
        None => bail!("Could not find any ndk version matching requirement {fuzzy_version_range}"),
    }
}

impl Command for Ndk {
    fn execute(self) -> Result<()> {
        match self.op {
            NdkOperation::Download(d) => {
                let manifest = get_android_manifest()?;

                let (_version, ndk) = fuzzy_match_ndk(&manifest, &d.version)?;
                download_ndk_version(ndk)?;
                return Ok(());
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
            NdkOperation::Resolve(r) => do_resolve(r)?,
            NdkOperation::Use(u) => do_use(u)?,
        }

        Ok(())
    }
}

fn do_use(u: UseArgs) -> Result<(), color_eyre::eyre::Error> {
    let version = match u.installed_only {
        true => {
            let version_req = VersionReq::parse(&u.version)?;
            // find latest NDK that satisfies requirement
            let ndk_installed_path = get_combine_config()
                .get_ndk_installed()
                .into_iter()
                .flatten()
                .sorted_by(|a, b| a.file_name().cmp(b.file_name()))
                .rev()
                .find(|n| {
                    Version::parse(n.file_name().to_str().unwrap())
                        .is_ok_and(|v| version_req.matches(&v))
                })
                .ok_or_else(|| {
                    eyre!("No NDK version installed that satisfies {version_req}")
                })?;

            ndk_installed_path.file_name().to_str().unwrap().to_string()
        }
        // allow any NDK version online
        false => {
            let manifest = get_android_manifest()?;
            fuzzy_match_ndk(&manifest, &u.version)?.0
        }
    };
    let mut package = PackageConfig::read(".")?;
    let req = match u.strict {
        true => format!("={version}"),
        false => format!("^{version}"),
    };
    package.workspace.ndk = Some(VersionReq::parse(&req)?);
    package.write(".")?;
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
    Ok(())
}

fn do_resolve(r: ResolveArgs) -> Result<(), color_eyre::eyre::Error> {
    let package = PackageConfig::read(".")?;
    let ndk_requirement = package.workspace.ndk.as_ref();
    let Some(ndk_requirement) = ndk_requirement else {
        bail!("No NDK requirement set in project")
    };
    let ndk_installed_path_opt = get_combine_config()
        .get_ndk_installed()
        .into_iter()
        .flatten()
        .sorted_by(|a, b| a.file_name().cmp(b.file_name()))
        .rev() // descending
        .find(|s| {
            Version::parse(s.file_name().to_str().unwrap())
                .is_ok_and(|version| ndk_requirement.matches(&version))
        });
    let ndk_installed_path: PathBuf = match ndk_installed_path_opt {
        // NDK Found, unwrap
        Some(ndk_installed_path) => ndk_installed_path.path().into(),
        // download
        None if r.download => {
            let manifest = get_android_manifest()?;
            let (_version, ndk) = range_match_ndk(&manifest, ndk_requirement)?;

            download_ndk_version(ndk)?
        }
        // error
        _ => {
            let mut report =
                eyre!("No NDK version installed that satisfies {ndk_requirement}")
                    .note("-d/--download not set, not downloading!");

            // look up a version suitable to work with
            // allow this to work offline by handling safely
            let manifest = get_android_manifest().ok();
            let mut suggested_version = manifest
                .as_ref()
                .and_then(|manifest| range_match_ndk(manifest, ndk_requirement).ok());

            if let Some((suggested_version, _)) = &mut suggested_version {
                report =
                    report.suggestion(format!("qpm ndk download {suggested_version}"));
            }

            return Err(report);
        }
    };

    // apply the NDK to the environment
    apply_ndk(&ndk_installed_path)?;
    Ok(())
}

// apply NDK to project and write
fn apply_ndk(ndk_installed_path: &Path) -> Result<(), color_eyre::eyre::Error> {
    let mut ndk_file = File::create("./ndkpath.txt").context("Unable to open ndkpath.txt")?;
    write!(ndk_file, "{}", ndk_installed_path.to_str().unwrap())?;
    ndk_file.flush()?;
    println!("{}", ndk_installed_path.to_str().unwrap());
    Ok(())
}
