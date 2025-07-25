use std::{fs::File, io::BufReader, path::Path};

use color_eyre::eyre::{Context, Result};
use qpm_package::models::qpackages::QPackagesPackage;

use crate::utils::json;

pub const QPACKAGES_JSON: &str = "qpackages.json";

pub trait QPackageExtensions: Sized {
    /// Checks if the QPackage exists in the given directory.
    fn exists<P: AsRef<Path>>(dir: P) -> bool;

    /// Reads the QPackage from the given directory.
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self>
    where
        Self: std::marker::Sized;

    /// Writes the QPackage to the given directory.
    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()>;
}

impl QPackageExtensions for QPackagesPackage {
    fn exists<P: AsRef<Path>>(dir: P) -> bool {
        let path: std::path::PathBuf = dir.as_ref().join(QPACKAGES_JSON);
        path.exists()
    }

    fn read<P: AsRef<Path>>(dir: P) -> Result<Self>
    where
        Self: std::marker::Sized {
        let path = dir.as_ref().join(QPACKAGES_JSON);
        let file = File::open(&path).with_context(|| format!("{path:?} does not exist"))?;
        let res = json::json_from_reader_fast::<_, Self>(BufReader::new(file))
            .with_context(|| format!("Unable to read QPackage at {path:?}"))?;

        Ok(res)
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let path = dir.as_ref().join(QPACKAGES_JSON);
        
        let serialized = serde_json::to_string_pretty(self)
            .context("Failed to serialize QPackage")?;
        std::fs::write(path, serialized)
            .context("Failed to write QPackage to file")?;
        Ok(())
    }
}