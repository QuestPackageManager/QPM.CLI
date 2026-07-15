use std::io::Cursor;

use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use owo_colors::OwoColorize;
use qpm_package::models::{
    package::PackageConfig, qpackages::QPackagesPackage, shared_package::SharedPackageConfig,
};
use sha2::{Digest, Sha256};

use crate::{
    models::qpkg_file::QpkgFile,
    repository::{Repository, qpackages::QPMRepository},
    services::network::download_bytes,
    terminal::colors::QPMColor,
};

/// Validates a package + its qpkg artifact before publishing, then submits it to a backend
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
        package: PackageConfig,
        shared_package: &SharedPackageConfig,
        qpkg_url: String,
        repo: &impl Repository,
    ) -> Result<Self> {
        check_dependencies(&package, repo, shared_package)?;

        let qpkg_cursor = Self::download_and_verify_qpkg(&qpkg_url, &package)
            .context("Validating QPKG failed")?;

        let result = Sha256::digest(qpkg_cursor.get_ref());
        let qpkg_checksum = hex::encode(result);

        Ok(Self {
            package,
            qpkg_url,
            qpkg_checksum,
        })
    }

    fn download_and_verify_qpkg(
        qpkg_url: &str,
        package: &PackageConfig,
    ) -> Result<Cursor<bytes::BytesMut>> {
        // TODO: What if the URL is not accessible due to Authorization or rate limits?
        let bytes = download_bytes(qpkg_url).context(
            "Downloading qpkg file failed. QPKG URL must be accessible at time of publishing",
        )?;

        let cursor = Cursor::new(bytes);

        let qpkg_file = QpkgFile::open(cursor).context("Failed to read QPKG")?;
        if qpkg_file.manifest().config != *package {
            bail!(
                "QPKG config mismatch. Expected{:#?}\nGot {:#?}",
                package,
                qpkg_file.manifest().config
            );
        }

        Ok(qpkg_file.into_inner())
    }

    /// Submits the validated package to the qpackages.dev backend using the given auth token
    pub fn publish_to_qpackages(&self, auth_token: &str) -> Result<QPackagesPackage> {
        let qpackage = QPackagesPackage {
            config: self.package.clone(),
            qpkg_checksum: Some(self.qpkg_checksum.clone()),
            qpkg_url: self.qpkg_url.clone(),
        };

        QPMRepository::publish_package(&qpackage, auth_token)?;

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
