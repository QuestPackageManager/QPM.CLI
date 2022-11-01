use std::path::Path;

use color_eyre::Result;
use semver::Version;

use qpm_package::models::{backend::PackageVersion, dependency::SharedPackageConfig};

pub mod local;
pub mod multi;
pub mod qpackages;

pub trait Repository {
    fn get_package_names(&self) -> Result<Vec<String>>;
    fn get_package_versions(&self, id: &str) -> Result<Option<Vec<PackageVersion>>>;
    fn get_package(&self, id: &str, version: &Version) -> Result<Option<SharedPackageConfig>>;
    fn get_package_and_memcache(
        &mut self,
        id: &str,
        version: &Version,
    ) -> Result<Option<SharedPackageConfig>> {
        let result = self.get_package(id, version)?;
        if let Some(p) = &result {
            self.add_to_db_cache(p.clone(), false)?;
        }
        Ok(result)
    }

    fn add_to_db_cache(&mut self, config: SharedPackageConfig, permanent: bool) -> Result<()>;

    // downloads if not in cache
    fn pull_from_cache(&mut self, config: &SharedPackageConfig, target: &Path) -> Result<()>;
}
