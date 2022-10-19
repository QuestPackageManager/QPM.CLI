use color_eyre::Result;
use semver::Version;

use qpm_package::models::{dependency::{SharedPackageConfig}, backend::PackageVersion};

pub mod local;
pub mod multi;
pub mod qpackages;

pub trait Repository {
    fn get_package_names(&self) -> Result<Vec<String>>;
    fn get_package_versions(&self, id: &str) -> Result<Option<Vec<PackageVersion>>>;

    fn get_package(&self, id: &str, version: &Version) -> Result<Option<SharedPackageConfig>>;

    fn add_to_cache(&mut self, config: SharedPackageConfig, permanent: bool) -> Result<()>;
}
