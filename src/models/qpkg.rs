use std::{fs::File, io::BufReader, path::Path};

use color_eyre::eyre::{Context, Result};
use qpm_package::models::qpkg::{QPKG_JSON, QPkg};

use crate::utils::json;

pub trait QPkgExtensions: Sized {
    fn exists<P: AsRef<Path>>(dir: P) -> bool;

    fn read<P: AsRef<Path>>(dir: P) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()>;
}

impl QPkgExtensions for QPkg {
    fn exists<P: AsRef<Path>>(dir: P) -> bool {
        let path = dir.as_ref().join(QPKG_JSON);
        path.exists()
    }

    fn read<P: AsRef<Path>>(dir: P) -> Result<Self>
    where
        Self: std::marker::Sized,
    {
        let path = dir.as_ref().join(QPKG_JSON);
        let file = File::open(&path).with_context(|| format!("{path:?} does not exist"))?;
        let res = json::json_from_reader_fast::<_, Self>(BufReader::new(file))
            .with_context(|| format!("Unable to read PackageConfig at {path:?}"))?;

        Ok(res)
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let path = dir.as_ref().join(QPKG_JSON);

        let serialized = serde_json::to_string_pretty(self).context("Failed to serialize QPkg")?;
        std::fs::write(path, serialized).context("Failed to write QPkg to file")?;
        Ok(())
    }
}
