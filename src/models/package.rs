use std::{fs::File, io::BufReader, path::Path};

use color_eyre::{
    eyre::{anyhow, bail, Context},
    Result,
};
use itertools::Itertools;
use qpm_package::models::{
    dependency::{Dependency, SharedDependency, SharedPackageConfig},
    extra::DependencyLibType,
    package::PackageConfig,
};
use qpm_qmod::models::mod_json::{ModDependency, ModJson};
use semver::VersionReq;

use crate::{
    repository::Repository, resolver::dependency::resolve, terminal::colors::QPMColor, utils::json,
};

use super::{package_dependeny::PackageDependencyExtensions, toolchain};

pub const PACKAGE_FILE_NAME: &str = "qpm.json";
pub const SHARED_PACKAGE_FILE_NAME: &str = "qpm.shared.json";

pub trait PackageConfigExtensions {
    fn exists<P: AsRef<Path>>(dir: P) -> bool;
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()>;
    fn run_if_version(
        &self,
        req: &VersionReq,
        func: impl FnOnce() -> color_eyre::Result<()>,
    ) -> color_eyre::Result<()>;
    fn matches_version(&self, req: &VersionReq) -> bool;

    fn validate(&self) -> color_eyre::Result<()>;

    fn get_static_lib_out(&self) -> Result<&Path>;
    fn get_dynamic_lib_out(&self) -> Result<&Path>;
}
pub trait SharedPackageConfigExtensions: Sized {
    fn resolve_from_package(
        config: PackageConfig,
        repository: &impl Repository,
    ) -> Result<(Self, Vec<SharedPackageConfig>)>;

    fn to_mod_json(self) -> ModJson;

    fn try_write_toolchain(&self, repo: &impl Repository) -> Result<()>;
    fn verify(&self) -> color_eyre::Result<()>;
}

impl PackageConfigExtensions for PackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let path = dir.as_ref().join(PACKAGE_FILE_NAME);
        let file = File::open(&path).with_context(|| format!("{path:?} does not exist"))?;
        let res = json::json_from_reader_fast::<_, Self>(BufReader::new(file))
            .with_context(|| format!("Unable to read PackageConfig at {path:?}"))?;
        res.validate()?;

        Ok(res)
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let path = dir.as_ref().join(PACKAGE_FILE_NAME);
        let file = File::create(&path).with_context(|| format!("{path:?} cannot be written"))?;

        serde_json::to_writer_pretty(file, self)
            .with_context(|| format!("Unable to write PackageConfig at {path:?}"))?;
        Ok(())
    }

    fn exists<P: AsRef<Path>>(dir: P) -> bool {
        dir.as_ref().with_file_name(PACKAGE_FILE_NAME).exists()
    }

    fn run_if_version(
        &self,
        req: &VersionReq,
        func: impl FnOnce() -> color_eyre::Result<()>,
    ) -> color_eyre::Result<()> {
        if req.matches(&self.version) {
            return func();
        }

        Ok(())
    }
    fn matches_version(&self, req: &VersionReq) -> bool {
        req.matches(&self.version)
    }

    fn validate(&self) -> color_eyre::Result<()> {
        let default = Self::default();

        if self.version.major != default.version.major {
            eprintln!(
                "Warning: using outdate qpm schema. Current {} Latest: {:?}",
                self.version, default.version
            );
        }

        let headers_only = self
            .info
            .additional_data
            .headers_only
            .unwrap_or(false);
        let dynamic_lib_out = &self.info.additional_data.dynamic_lib_out;
        let static_lib_out = &self.info.additional_data.static_lib_out;

        if !headers_only && dynamic_lib_out.is_none() && static_lib_out.is_none() {
            bail!(
                "{} nor {} are defined!",
                "qpm.shared.json::config::info::additionalData::dynamicLibOut".file_path_color(),
                "qpm.shared.json::config::info::additionalData::staticLibOut".file_path_color()
            );
        }

        Ok(())
    }

    fn get_static_lib_out(&self) -> Result<&Path> {
        let path = self
            .info
            .additional_data
            .static_lib_out
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "{} qpm.shared.json::info::additionalData::staticLibOut not defined",
                    self.info.id.dependency_id_color()
                )
            })?;

        Ok(path)
    }

    fn get_dynamic_lib_out(&self) -> Result<&Path> {
        let path = self
            .info
            .additional_data
            .dynamic_lib_out
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "{} qpm.shared.json::info::additionalData::dynamicLibOut not defined",
                    self.info.id.dependency_id_color()
                )
            })?;

        Ok(path)
    }
}
impl PackageConfigExtensions for SharedPackageConfig {
    fn read<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let path = dir.as_ref().join(SHARED_PACKAGE_FILE_NAME);
        let file = File::open(&path).with_context(|| format!("{path:?} not found"))?;
        json::json_from_reader_fast(BufReader::new(file))
            .with_context(|| format!("Unable to read SharedPackageConfig at {path:?}"))
    }

    fn write<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let path = dir.as_ref().join(SHARED_PACKAGE_FILE_NAME);
        let file = File::create(&path).with_context(|| format!("{path:?} cannot be written"))?;

        serde_json::to_writer_pretty(file, self)
            .with_context(|| format!("Unable to write PackageConfig at {path:?}"))?;
        Ok(())
    }
    fn exists<P: AsRef<Path>>(dir: P) -> bool {
        dir.as_ref().join(SHARED_PACKAGE_FILE_NAME).exists()
    }

    fn run_if_version(
        &self,
        req: &VersionReq,
        func: impl FnOnce() -> color_eyre::Result<()>,
    ) -> color_eyre::Result<()> {
        self.config.run_if_version(req, func)
    }

    fn matches_version(&self, req: &VersionReq) -> bool {
        self.config.matches_version(req)
    }

    fn validate(&self) -> color_eyre::Result<()> {
        self.config.validate()
    }

    fn get_static_lib_out(&self) -> Result<&Path> {
        self.config.get_static_lib_out()
    }

    fn get_dynamic_lib_out(&self) -> Result<&Path> {
        self.config.get_dynamic_lib_out()
    }
}

impl SharedPackageConfigExtensions for SharedPackageConfig {
    fn resolve_from_package(
        config: PackageConfig,
        repository: &impl Repository,
    ) -> Result<(Self, Vec<SharedPackageConfig>)> {
        let resolved_deps = resolve(&config, repository)?.collect_vec();

        let restored_dependencies = resolved_deps
            .iter()
            .map(|shared_dep_config| {
                let declared_dep = config
                    .dependencies
                    .iter()
                    .find(|declared_dep| declared_dep.id == shared_dep_config.config.info.id);

                (shared_dep_config, declared_dep)
            })
            .map(|(shared_dep_config, declared_dep)| {
                let restored_lib_type = match declared_dep {
                    // infer the lib type
                    // if explicitly set, will use that
                    Some(declared_dep) => {
                        declared_dep.infer_lib_type(&shared_dep_config.config.info.additional_data)
                    }

                    // if not declared directly, just restore as header only
                    None => DependencyLibType::HeaderOnly,
                };
                SharedDependency {
                    dependency: Dependency {
                        id: shared_dep_config.config.info.id.clone(),
                        version_range: VersionReq::parse(&format!(
                            "={}",
                            shared_dep_config.config.info.version
                        ))
                        .expect("Unable to parse version"),
                        additional_data: shared_dep_config.config.info.additional_data.clone(),
                    },
                    version: shared_dep_config.config.info.version.clone(),
                    restored_lib_type,
                }
            })
            .collect();

        Ok((
            SharedPackageConfig {
                config,
                restored_dependencies,
            },
            resolved_deps,
        ))
    }

    fn to_mod_json(self) -> ModJson {
        //        Self {
        //     id: dep.id,
        //     version_range: dep.version_range,
        //     mod_link: dep.additional_data.mod_link,
        // }

        let local_deps = &self.config.dependencies;

        // Only bundle mods that are not specifically excluded in qpm.json or if they're not header-only
        let required_deps: Vec<_> = self
            .restored_dependencies
            .iter()
            .filter(|shared_dep| {
                // find the actual dependency for the include qmod value
                let local_dep_opt = local_deps
                    .iter()
                    .find(|local_dep| local_dep.id == shared_dep.dependency.id);

                // if set, we will include qmod
                let include_qmod =
                    local_dep_opt.and_then(|local_dep| local_dep.additional_data.include_qmod);

                // don't include static deps or header only
                shared_dep.restored_lib_type == DependencyLibType::Shared &&

                // it's marked to be included, defaults to including ( same as dependencies with qmods )
                include_qmod.unwrap_or(true)
            })
            .collect();

        // downloadable mods links n stuff
        // mods that are header-only but provide qmods can be added as deps
        let mod_dependencies: Vec<ModDependency> = required_deps
            .iter()
            // Removes any dependency without a qmod link
            .filter(|&shared_dep| shared_dep.dependency.additional_data.mod_link.is_some())
            .map(|shared_dep| ModDependency {
                version_range: shared_dep.dependency.version_range.clone(),
                id: shared_dep.dependency.id.clone(),
                mod_link: shared_dep.dependency.additional_data.mod_link.clone(),
            })
            .collect();

        // The rest of the mods to handle are not qmods, they are .so or .a mods
        // actual direct lib deps
        let libs: Vec<String> = required_deps
            .iter()
            // look for mods with no qmods
            .filter(|dep| dep.dependency.additional_data.mod_link.is_none())
            .map(|dep| {
                dep.dependency
                    .additional_data
                    .dynamic_lib_out
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!(
                            "Dependency {} does not define dynamic lib out",
                            dep.dependency.id.dependency_id_color()
                        )
                    })
            })
            // get file name of dep
            .map(|path| path.file_name().unwrap().to_str().unwrap().to_string())
            .collect();

        ModJson {
            name: self.config.info.name.clone(),
            id: self.config.info.id.clone(),
            porter: None,
            version: self.config.info.version.to_string(),
            package_id: None,
            package_version: None,
            description: None,
            cover_image: None,
            is_library: None,
            dependencies: mod_dependencies,
            library_files: libs,
            ..Default::default()
        }
    }

    /// check if shared json is valid for publishing
    fn verify(&self) -> color_eyre::Result<()> {
        Ok(())
    }

    fn try_write_toolchain(&self, repo: &impl Repository) -> Result<()> {
        let Some(toolchain_path) = self.config.info.additional_data.toolchain_out.as_ref() else {
            return Ok(());
        };

        toolchain::write_toolchain_file(self, repo, toolchain_path)?;

        Ok(())
    }
}
