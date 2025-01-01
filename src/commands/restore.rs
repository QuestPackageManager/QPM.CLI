use std::{env, fs::File, io::Read, path::Path};

use clap::Args;

use color_eyre::{
    eyre::{bail, eyre, Context, ContextCompat, Result},
    Section,
};
use itertools::Itertools;
use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};
use semver::Version;

use crate::{
    models::{
        config::get_combine_config,
        package::{
            PackageConfigExtensions, SharedPackageConfigExtensions, SHARED_PACKAGE_FILE_NAME,
        },
    },
    repository::{self, Repository},
    resolver::dependency,
    terminal::colors::QPMColor,
};

use super::Command;

#[derive(Args)]
pub struct RestoreCommand {
    #[clap(default_value = "false", long, short)]
    update: bool,

    #[clap(long, default_value = "false")]
    offline: bool,
}

#[cfg(feature = "gitoxide")]
pub(crate) fn is_ignored() -> bool {
    gix::open(".").is_ok_and(|r| {
        let Ok(index) = r.index() else { return false };

        let excludes = r.excludes(&index, None, Default::default());

        excludes.is_ok_and(|mut attribute| {
            attribute
                .at_path(
                    SHARED_PACKAGE_FILE_NAME,
                    Some(gix::index::entry::Mode::FILE),
                )
                .is_ok_and(|e| e.is_excluded())
        })
    })
}

#[cfg(not(feature = "gitoxide"))]
pub(crate) fn is_ignored() -> bool {
    false
}

impl Command for RestoreCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        // optionally does not exist
        let mut shared_package_opt = SharedPackageConfig::exists(".")
            .then(|| SharedPackageConfig::read("."))
            .transpose()?;

        let mut repo = repository::useful_default_new(self.offline)?;

        // only update if:
        // manually
        // no shared.qpm.json
        // dependencies have been updated
        let unlocked = self.update
            || shared_package_opt.is_none()
            || shared_package_opt.as_ref().is_some_and(|shared_package| {
                shared_package.config.dependencies != package.dependencies
            });

        if !unlocked && is_ignored() {
            eprintln!(
                "It seems that the current repository has {SHARED_PACKAGE_FILE_NAME} ignored. "
            );
            eprintln!("Please commit it to avoid inconsistent dependency resolving. git add {SHARED_PACKAGE_FILE_NAME} --force");
        }

        if unlocked && env::var("CI") == Ok("true".to_string()) {
            eprintln!("Running in CI and using unlocked resolve, this seems like a bug!");
            eprintln!("Make sure {SHARED_PACKAGE_FILE_NAME} is not gitignore'd and is comitted in the repository");
        }

        let resolved_deps = match &mut shared_package_opt {
            // locked resolve
            // only if shared_package is Some() and locked
            Some(shared_package) if !unlocked => {
                // if the same, restore as usual
                println!("Using lock file for restoring");

                // update config
                shared_package.config = package;

                // https://discord.com/channels/994470435100033074/994630741235347566/1265083186157715538
                // make additional data update (for local installs)
                shared_package
                    .restored_dependencies
                    .iter_mut()
                    .try_for_each(|d| -> color_eyre::Result<()> {
                        let package: SharedPackageConfig = repo
                            .get_package(&d.dependency.id, &d.version)
                            .with_context(|| {
                                format!(
                                    "Unable to fetch {}:{}",
                                    d.dependency.id.dependency_id_color(),
                                    d.version.version_id_color()
                                )
                            })?
                            .wrap_err_with(|| {
                                format!(
                                    "Package {}:{} does not exist",
                                    d.dependency.id.dependency_id_color(),
                                    d.version.version_id_color()
                                )
                            })?;
                        d.dependency.additional_data = package.config.info.additional_data;
                        Ok(())
                    })?;
                dependency::locked_resolve(shared_package, &repo)?.collect_vec()
            }
            // Unlocked resolve
            _ => {
                println!("Resolving packages");

                let (spc_result, restored_deps) =
                    SharedPackageConfig::resolve_from_package(package, &repo)?;
                // update shared_package
                shared_package_opt = Some(spc_result);

                restored_deps
            }
        };

        // write the ndk path to a file if available
        let _config = get_combine_config();

        let shared_package = shared_package_opt.expect("SharedPackage is None somehow!");

        // always write to reflect config changes
        dependency::restore(".", &shared_package, &resolved_deps, &mut repo)?;
        shared_package.write(".")?;

        validate_ndk(&shared_package.config)?;

        Ok(())
    }
}

pub fn validate_ndk(package: &PackageConfig) -> Result<()> {
    let Some(ndk_req) = package.workspace.ndk.as_ref() else {
        return Ok(());
    };

    let mut ndk_path_str = String::new();

    // early return, the file doesn't exist nothing to validate
    let ndk_path = Path::new("./ndkpath.txt");
    if ndk_path.exists() {
        let mut ndk_file = File::open(ndk_path)?;

        ndk_file.read_to_string(&mut ndk_path_str)?;
        // validate environment variable if possible
    } else if let Some(ndk_path_env) =
        std::env::var_os("ANDROID_NDK_HOME").or_else(|| std::env::var_os("ANDROID_NDK_LATEST_HOME"))
    {
        ndk_path_str = ndk_path_env.to_str().unwrap().to_string();
    }

    let ndk_path = Path::new(ndk_path_str.trim());
    if !ndk_path.exists() {
        bail!("NDK Path {} does not exist!", ndk_path.display());
    }

    let ndk_version_str = ndk_path.file_name().unwrap().to_str().unwrap();
    let Ok(ndk_version) = Version::parse(ndk_version_str) else {
        println!("Unable to validate {ndk_version_str} is a valid NDK version, skipping");
        return Ok(());
    };

    if !ndk_req.matches(&ndk_version) {
        return Err(
            eyre!("NDK Version {ndk_version} does not satisfy {ndk_req}")
                .suggestion("qpm ndk resolve".to_string()),
        );
    }

    Ok(())
}
