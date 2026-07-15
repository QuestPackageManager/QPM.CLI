//! Build command: Compiles a mod using the configured build script.
//!
//! Process:
//! 1. Restore dependencies if qpm2.json differs from qpm2.shared.json
//! 2. Resolve NDK if --ndk-resolve flag is set
//! 3. Execute build script from workspace.scripts.build
//! 4. Optionally create QMOD package if --qmod flag is set

use std::path::PathBuf;

use clap::Args;
use color_eyre::eyre::{Context, ContextCompat};
use qpm_package::{
    extensions::workspace::WorkspaceConfigExtensions,
    models::{package::PackageConfig, shared_package::SharedPackageConfig},
};

use crate::{
    commands::{
        ndk::{Ndk, ResolveArgs},
        qmod::zip,
        scripts,
    },
    models::package::PackageConfigExtensions,
    repository::{self, local::FileRepository},
    services::restore::PackageRestorer,
    terminal::colors::QPMColor,
};

use super::Command;

/// Builds mod: restores dependencies, runs build script, copies binaries
#[derive(Args, Clone, Debug)]
pub struct BuildCommand {
    pub args: Option<Vec<String>>,

    /// Offline mode repository access
    #[clap(short, long, default_value = "false")]
    pub offline: bool,

    /// Where to output the build artifacts
    #[clap(short, long)]
    pub out_dir: Option<PathBuf>,

    /// Whether to zip the output as a qmod
    /// This does not set the QMOD url
    #[clap(long, default_value = "false")]
    pub qmod: bool,

    /// The build script to run
    #[clap(long)]
    pub build_script: Option<String>,

    /// If to resolve NDK dependencies
    #[clap(long, default_value = "false")]
    pub ndk_resolve: bool,
}

impl Command for BuildCommand {
    /// Executes the build workflow: restore -> resolve NDK -> build -> qmod (optional)
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        let mut shared_package = SharedPackageConfig::read(".")?;

        let out_dir = self
            .out_dir
            .unwrap_or_else(|| FileRepository::build_path(&package.dependencies_directory));

        let build_script = match self.build_script {
            Some(script) => Some(
                package
                    .workspace
                    .scripts
                    .get(&script)
                    .context(format!("Failed to find build script {script} in package"))?,
            ),
            None => package.workspace.get_build(),
        };

        let args = self.args.unwrap_or_default();

        let mut repo = repository::useful_default_new(self.offline)?;

        // TODO: sanity check. This collapses the old "is this triplet already restored"
        // gate (comparing shared_package.restored_triplet against the requested triplet)
        // into a full config-drift comparison, matching restore.rs's is_modified. Confirm
        // this is the desired trigger for re-resolving/restoring during build.
        if shared_package.config != package {
            println!("Restoring dependencies");

            shared_package.config = package.clone();
            let restorer = PackageRestorer::locked_resolve(&shared_package, &repo)?;
            restorer.restore(".", &mut repo)?;
            shared_package.write(".")?;

            if self.ndk_resolve {
                let ndk = Ndk {
                    op: crate::commands::ndk::NdkOperation::Resolve(ResolveArgs {
                        download: true,
                        ignore_missing: false,
                    }),
                    quiet: false,
                };

                ndk.execute()
                    .context("Failed to resolve NDK dependencies")?;
            }
        }

        // run build
        if let Some(build_script) = &build_script {
            println!("Building QPKG for {}", package.id.dependency_id_color());
            scripts::invoke_script(build_script, &args, &package)?;
        }

        // finally qmod
        if self.qmod {
            zip::execute_qmod_zip_operation(Default::default(), vec![out_dir.clone()])
                .context("Making qmod")?;
        }

        Ok(())
    }
}
