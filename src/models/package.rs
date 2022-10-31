use std::{fs::File, path::{Path}};

use color_eyre::Result;
use qpm_package::models::{package::PackageConfig, dependency::SharedPackageConfig};

pub trait PackageConfigExtensions {
    fn read(dir: &Path) -> Result<Self> where Self: std::marker::Sized;
    fn write(&self, dir: &Path) -> Result<()>;
}

impl PackageConfigExtensions for PackageConfig {
    fn read(dir: &Path) -> Result<Self> {
        let file = File::open(dir.with_file_name("qpm.json"))?;
        Ok(serde_json::from_reader(file)?)
    }

    fn write(&self, dir: &Path) -> Result<()> {
        let file = File::create(dir.with_file_name("qpm.json"))?;

        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}
impl PackageConfigExtensions for SharedPackageConfig {
    fn read(dir: &Path) -> Result<Self> {
        let file = File::open(dir.with_file_name("qpm.shared.json"))?;
        Ok(serde_json::from_reader(file)?)
    }

    fn write(&self, dir: &Path) -> Result<()> {
        let file = File::create(dir.with_file_name("qpm.shared.json"))?;

        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}
