use std::path::{Path, PathBuf};

use clap::Args;
use color_eyre::eyre::{Context, ContextCompat};
use itertools::Itertools;
use qpm_package::{
    extensions::workspace::WorkspaceConfigExtensions,
    models::{
        package::PackageConfig,
        shared_package::{SharedPackageConfig, SharedTriplet},
        triplet::{PackageTriplet, TripletId},
    },
};

use crate::{
    commands::{qmod::zip, scripts},
    models::package::PackageConfigExtensions,
    repository,
    resolver::dependency,
    terminal::colors::QPMColor,
};

use super::Command;

/// Templatr rust rewrite (implementation not based on the old one)
#[derive(Args, Clone, Debug)]
pub struct BuildCommand {
    pub args: Option<Vec<String>>,

    #[clap(short, long)]
    pub triplets: Option<Vec<String>>,

    #[clap(short, long, default_value = "false")]
    pub offline: bool,

    #[clap(short, long, default_value = "false")]
    pub out_dir: Option<PathBuf>,

    #[clap(long, default_value = "false")]
    pub qmod: bool,
}

impl Command for BuildCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;
        let mut shared_package = SharedPackageConfig::read(".")?;

        let out_dir = self
            .out_dir
            .unwrap_or_else(|| package.dependencies_directory.join("build"));

        let script = package
            .workspace
            .get_build()
            .context("Failed to get build script from workspace")?;

        let args = self.args.unwrap_or_default();

        let mut repo = repository::useful_default_new(self.offline)?;

        let mut build_triplet = |triplet_id: &TripletId,
                                 triplet: &PackageTriplet|
         -> color_eyre::Result<()> {
            let shared_triplet = shared_package
                .locked_triplet
                .get(triplet_id)
                .context("Failed to get triplet settings")?;

            println!(
                "Restoring dependencies for triplet {}",
                triplet_id.triplet_id_color()
            );
            // restore for triplet
            let resolved_deps =
                dependency::locked_resolve(&shared_package, &repo, shared_triplet)?.collect_vec();

            // now restore
            dependency::restore(".", &shared_package, triplet_id, &resolved_deps, &mut repo)?;
            shared_package.restored_triplet = triplet_id.clone();
            shared_package.write(".")?;

            // run builld
            println!(
                "Building QPKG for triplet {}",
                triplet_id.triplet_id_color()
            );
            scripts::invoke_script(script, &args, &package, triplet_id)?;

            let triplet_dir = out_dir.join(&triplet_id.0);

            // now copy binaries
            copy_bins(
                &triplet_dir,
                &triplet.out_binaries.clone().unwrap_or_default(),
            )?;

            // finally qmod
            if self.qmod {
                zip::execute_qmod_zip_operation(Default::default(), &[&triplet_dir])
                    .context("Making triplet qmod")?;
            }
            Ok(())
        };

        match self.triplets {
            Some(triplets) => {
                // build all triplets
                for triplet_id in triplets {
                    println!("Building triplet {}", triplet_id.triplet_id_color());

                    let triplet_id = TripletId(triplet_id);
                    let triplet = package
                        .triplets
                        .get_triplet(&triplet_id)
                        .context(format!("Failed to get triplet {triplet_id} from package"))?;

                    build_triplet(&triplet_id, triplet).with_context(|| {
                        format!("Failed to build triplet {}", triplet_id.triplet_id_color())
                    })?;
                }
            }
            None => {
                // build all triplets
                for (triplet_id, triplet) in package.triplets.iter_triplets() {
                    println!("Building triplet {}", triplet_id.triplet_id_color());
                    build_triplet(&triplet_id, triplet.as_ref()).with_context(|| {
                        format!("Failed to build triplet {}", triplet_id.triplet_id_color())
                    })?;
                }
            }
        }

        Ok(())
    }
}

fn copy_bins(triplet_dir: &Path, out_binaries: &[PathBuf]) -> color_eyre::Result<()> {
    for binary in out_binaries {
        // create the output directory if it doesn't exist
        let out_path = triplet_dir.join(binary);

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
                triplet_dir.display()
            )
        })?;
    }
    Ok(())
}
