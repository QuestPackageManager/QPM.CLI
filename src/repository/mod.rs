

use color_eyre::Result;
use semver::Version;

use qpm_package::models::{backend::PackageVersion, dependency::SharedPackageConfig, package::PackageConfig};

pub mod local;
pub mod multi;
pub mod qpackages;

pub trait Repository {
    fn get_package_names(&self) -> Result<Vec<String>>;
    fn get_package_versions(&self, id: &str) -> Result<Option<Vec<PackageVersion>>>;
    fn get_package(&self, id: &str, version: &Version) -> Result<Option<SharedPackageConfig>>;
    // add to the db cache
    // this just stores the shared config itself, not the package
    fn add_to_db_cache(&mut self, config: SharedPackageConfig, permanent: bool) -> Result<()>;

    // downloads if not in cache
    // What if we wanted to have a qpackages mirror or a new backend? ;)
    // Does not download dependencies
    fn download_to_cache(&mut self, config: &PackageConfig) -> Result<()>;

    fn write_repo(&self) -> Result<()>;
}
