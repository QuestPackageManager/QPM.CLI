use std::path::{Path, PathBuf};

use clap::Args;
use color_eyre::eyre::{Context, ContextCompat};
use itertools::Itertools;
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
    resolver::dependency,
    terminal::colors::QPMColor,
};

use super::Command;

/// Templatr rust rewrite (implementation not based on the old one)
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

            let resolved_deps = dependency::locked_resolve(&shared_package, &repo)?.collect_vec();

            shared_package.config = package.clone();
            dependency::restore(".", &shared_package, &resolved_deps, &mut repo)?;
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

        // now copy binaries
        copy_bins(&out_dir, &package.workspace.out_binaries.clone().unwrap_or_default())?;

        // finally qmod
        if self.qmod {
            zip::execute_qmod_zip_operation(Default::default(), vec![out_dir.clone()])
                .context("Making qmod")?;
        }

        Ok(())
    }
}

fn copy_bins(out_dir: &Path, out_binaries: &[PathBuf]) -> color_eyre::Result<()> {
    for binary in out_binaries {
        println!(
            "Copying binary {} to output directory {}",
            binary.display().file_path_color(),
            out_dir.display().file_path_color()
        );

        // create the output directory if it doesn't exist
        let out_path = out_dir.join(binary.file_name().unwrap_or_default());

        std::fs::create_dir_all(out_path.parent().unwrap()).with_context(|| {
            format!(
                "Failed to create output directory {}",
                out_path.parent().unwrap().display()
            )
        })?;

        std::fs::copy(binary, &out_path).with_context(|| {
            format!(
                "Failed to copy binary {} to output directory {}",
                binary.display(),
                out_dir.display()
            )
        })?;
    }
    Ok(())
}
