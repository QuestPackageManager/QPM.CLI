use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
};

use clap::Args;
use color_eyre::eyre::Context;
use qpm_package::models::{
    package::PackageConfig,
    qpkg::{QPKG_JSON, QPkg, QPkgTripletInfo},
    triplet::TripletId,
};
use zip::{ZipWriter, write::FileOptions};

use crate::{
    models::package::PackageConfigExtensions, repository::local::FileRepository,
    terminal::colors::QPMColor,
};

use super::Command;

/// Templatr rust rewrite (implementation not based on the old one)
#[derive(Args, Clone, Debug)]
pub struct QPkgCommand {
    #[clap(short, long)]
    pub bin_dir: Option<String>,

    #[clap(short, long)]
    pub triplets: Option<Vec<String>>,

    qpkg_output: Option<PathBuf>,
}

impl Command for QPkgCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;

        let out = self
            .qpkg_output
            .as_deref()
            .unwrap_or(Path::new(&package.id.0))
            .with_extension("qpkg");

        let file = std::fs::File::create(out)?;
        let mut zip = ZipWriter::new(file);

        let options = FileOptions::<()>::default();

        // add shared directory
        zip.add_directory_from_path(&package.shared_directory, options)
            .context("Failed to add shared directory to QPKG zip")?;

        for entry in walkdir::WalkDir::new(&package.shared_directory)
            .min_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            // remove the shared directory prefix from the path
            let rel_path = entry
                .path()
                .strip_prefix(&package.shared_directory)
                .unwrap();

            zip.start_file_from_path(rel_path, options)
                .with_context(|| format!("Failed to add file {:?} to QPKG zip", rel_path))?;

            let bytes = std::fs::read(entry.path()).context("Failed to read shared file")?;
            zip.write_all(&bytes)
                .context("Failed to write shared file to QPKG zip")?;
        }

        let build_dir = self
            .bin_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| FileRepository::build_path(&package.dependencies_directory));

        let triplets: HashMap<TripletId, QPkgTripletInfo> = package
            .triplets
            .iter_triplets()
            // Filter triplets based on the provided triplet IDs
            .filter(|(triplet_id, _)| {
                self.triplets.is_none() || self.triplets.as_ref().unwrap().contains(&triplet_id.0)
            })
            .filter_map(|(triplet_id, triplet)| {
                // extern/build/{triplet_id}/
                let triplet_dir = build_dir.join(&triplet_id.0);

                let binaries = triplet.out_binaries.clone()?;
                for binary in &binaries {
                    // extern/build/{triplet_id}/{binary}
                    let binary_built = triplet_dir.join(binary);

                    if !binary_built.exists() {
                        panic!(
                            "Binary {} for triplet {} does not exist (looking in {}). `qpm2 build` must be run first.",
                            binary.display(),
                            triplet_id.triplet_id_color(),
                            binary_built.display()
                        );
                    }

                    zip.start_file_from_path(&binary_built, options)
                        .expect("Failed to start file in QPKG zip");

                    let bytes = std::fs::read(&binary_built).expect("Failed to read binary file");
                    zip.write_all(&bytes)
                        .expect("Failed to write binary file to QPKG zip");
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
