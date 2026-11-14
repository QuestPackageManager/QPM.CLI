use std::io::BufReader;

use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use owo_colors::OwoColorize;
use qpm_package::models::{
    package::PackageConfig, qpackages::QPackagesPackage, shared_package::SharedPackageConfig,
};

use crate::{
    models::qpkg_file::QpkgFile,
    repository::{Repository, qpackages::QPMRepository},
    services::network::download_to_tempfile_hashed,
    terminal::colors::QPMColor,
};

/// Validates a package + its qpkg artifact before publishing, then submits it to a backend
#[derive(Debug)]
pub struct PackagePublisher {
    package: PackageConfig,
    qpkg_url: String,
    qpkg_checksum: String,
}

impl PackagePublisher {
    /// Validates that every dependency in `shared_package` is still resolvable against
    /// `repo`, downloads the qpkg at `qpkg_url` and checks its embedded config matches
    /// `package` exactly, then computes its checksum.
    pub fn validate(
        agent: &ureq::Agent,
        package: PackageConfig,
        shared_package: &SharedPackageConfig,
        qpkg_url: String,
        repo: &impl Repository,
    ) -> Result<Self> {
        check_dependencies(&package, repo, shared_package)?;

        let qpkg_checksum = Self::download_and_verify_qpkg(agent, &qpkg_url, &package)
            .context("Validating QPKG failed")?;

        Ok(Self {
            package,
            qpkg_url,
            qpkg_checksum,
        })
    }

    /// Returns the hex sha256 of the downloaded archive
    fn download_and_verify_qpkg(
        agent: &ureq::Agent,
        qpkg_url: &str,
        package: &PackageConfig,
    ) -> Result<String> {
        // TODO: What if the URL is not accessible due to Authorization or rate limits?
        let (file, checksum) = download_to_tempfile_hashed(agent, qpkg_url).context(
            "Downloading qpkg file failed. QPKG URL must be accessible at time of publishing",
        )?;

        let qpkg_file = QpkgFile::open(BufReader::new(file)).context("Failed to read QPKG")?;
        if qpkg_file.manifest().config != *package {
            bail!(
                "QPKG config mismatch. Expected{:#?}\nGot {:#?}",
                package,
                qpkg_file.manifest().config
            );
        }

        Ok(checksum)
    }

    /// Submits the validated package to the qpackages.dev backend using the given auth token
    pub fn publish_to_qpackages(
        &self,
        qpackages: &QPMRepository,
        auth_token: &str,
    ) -> Result<QPackagesPackage> {
        let qpackage = QPackagesPackage {
            config: self.package.clone(),
            qpkg_checksum: Some(self.qpkg_checksum.clone()),
            qpkg_url: self.qpkg_url.clone(),
        };

        qpackages.publish_package(&qpackage, auth_token)?;

        Ok(qpackage)
    }
}

fn check_dependencies(
    package: &PackageConfig,
    repo: &impl Repository,
    shared_package: &SharedPackageConfig,
) -> Result<()> {
    let resolved_deps = &shared_package.restored_dependencies;
    for (dep_id, dep) in resolved_deps {
        if repo.get_package(dep_id, &dep.restored_version)?.is_none() {
            bail!(
                "dependency {} was not available on qpackages in the given version range",
                &dep_id.dependency_id_color()
            );
        };
    }
    for (dep_id, dependency) in &package.dependencies {
        // if we can not find any dependency that matches ID and version satisfies given range, then we are missing a dep
        let el = shared_package
            .restored_dependencies
            .get(dep_id)
            .with_context(|| {
                format!(
                    "Dependency {} not found in restored dependencies",
                    dep_id.dependency_id_color()
                )
            })?;

        // if version doesn't match range, panic
        if !dependency.version_range.matches(&el.restored_version) {
            bail!(
                "Restored dependency {} version ({}) does not satisfy stated range ({})",
                dep_id.dependency_id_color(),
                el.restored_version.red(),
                dependency.version_range.green()
            );
        }
    }

    Ok(())
}
