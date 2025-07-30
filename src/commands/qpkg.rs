use std::{collections::HashMap, path::{Path, PathBuf}};

use clap::Args;
use color_eyre::eyre::Context;
use qpm_package::models::{
    package::PackageConfig,
    qpkg::{QPKG_JSON, QPkg, QPkgTripletInfo},
    triplet::TripletId,
};
use zip::{ZipWriter, write::FileOptions};

use crate::{models::package::PackageConfigExtensions, terminal::colors::QPMColor};

use super::Command;

/// Templatr rust rewrite (implementation not based on the old one)
#[derive(Args, Clone, Debug)]
pub struct QPkgCommand {
    #[clap(short, long)]
    pub bin_dir: Option<String>,

    qpkg_output: Option<PathBuf>,
}

impl Command for QPkgCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;

        let out = self.qpkg_output.as_deref().unwrap_or(Path::new(&package.id.0)).with_extension("qpkg");

        let file = std::fs::File::create(out)?;
        let mut zip = ZipWriter::new(file);

        let options = FileOptions::<()>::default();

        // add shared directory
        zip.add_directory_from_path(&package.shared_directory, options)
            .context("Failed to add shared directory to QPKG zip")?;

        let build_dir = self
            .bin_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| package.dependencies_directory.join("build"));

        let triplets: HashMap<TripletId, QPkgTripletInfo> = package
            .triplets
            .iter_triplets()
            .filter_map(|(triplet_id, triplet)| {
                // extern/build/{triplet_id}/
                let triplet_dir = build_dir.join(&triplet_id.0);

                let binaries = triplet.out_binaries.clone()?;
                for binary in &binaries {
                    // extern/build/{triplet_id}/{binary}
                    let binary_built = triplet_dir.join(binary);

                    if !binary_built.exists() {
                        panic!(
                            "Binary {} for triplet {} does not exist",
                            binary.display(),
                            triplet_id.triplet_id_color()
                        );
                    }

                    zip.start_file_from_path(binary_built, options)
                        .expect("Failed to start file in QPKG zip");
                }

                Some((triplet_id, QPkgTripletInfo { files: binaries }))
            })
            .collect();

        let qpkg = QPkg {
            shared_dir: package.shared_directory.clone(),
            config: package,
            triplets,
        };

        zip.start_file(QPKG_JSON, options)
            .context("Failed to start file in QPKG zip")?;
        serde_json::to_writer(&mut zip, &qpkg).context("Failed to write QPKG JSON to zip")?;
        zip.finish()?;

        println!("Executing QPKG command...");
        Ok(())
    }
}
