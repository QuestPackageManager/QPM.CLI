use std::{fs::File, io::BufReader, path::PathBuf};

use clap::Args;
use color_eyre::eyre::{Context, bail};
use owo_colors::OwoColorize;
use qpm_package::models::{
    extra::PackageCompileOptions,
    package::{
        self, DependencyId, DependencyMap, PackageAdditionalData, PackageConfig, PackageDependency,
        QmodConfig, QmodDependencyMode,
    },
    workspace::WorkspaceConfig,
};
use qpm_package_v1::models::{
    extra::CompileOptions as CompileOptionsV1, package::PackageConfig as PackageConfigV1,
    package::PackageDependency as PackageDependencyV1,
};

use crate::{models::package::PackageConfigExtensions, utils::json};

use super::Command;

const QPM1_JSON: &str = "qpm.json";

/// Migrate a legacy qpm.json (QPM v1) into a qpm2.json (QPM v2) package config
#[derive(Args, Debug, Clone)]
pub struct MigrateCommand {
    /// Path to the legacy qpm.json to migrate from, defaults to ./qpm.json
    #[clap(long = "input", short = 'i')]
    pub input: Option<PathBuf>,

    /// Overwrite an existing qpm2.json if one is already present
    #[clap(long, default_value = "false")]
    pub force: bool,
}

impl Command for MigrateCommand {
    fn execute(self) -> color_eyre::Result<()> {
        if !self.force && PackageConfig::exists(".") {
            bail!(
                "qpm2.json already exists, refusing to overwrite it. Use {} to overwrite it anyway.",
                "--force".yellow()
            );
        }

        let input = self.input.unwrap_or_else(|| PathBuf::from(QPM1_JSON));

        let file = File::open(&input).with_context(|| format!("{input:?} does not exist"))?;
        let qpm1: PackageConfigV1 = json::json_from_reader_fast(BufReader::new(file))
            .with_context(|| format!("Unable to parse legacy package config at {input:?}"))?;

        let qpm2 = convert(qpm1);
        qpm2.write(".")?;

        println!(
            "Migrated {} to {}",
            input.display().to_string().yellow(),
            "qpm2.json".green()
        );

        Ok(())
    }
}

fn convert(qpm1: PackageConfigV1) -> PackageConfig {
    let mut dependencies = DependencyMap::new();
    let mut dev_dependencies = DependencyMap::new();

    for dep in &qpm1.dependencies {
        let id = DependencyId(dep.id.clone());
        let converted = convert_dependency(dep);

        if dep.additional_data.is_private.unwrap_or(false) {
            dev_dependencies.insert(id, converted);
        } else {
            dependencies.insert(id, converted);
        }
    }

    let headers_only = qpm1.info.additional_data.headers_only.unwrap_or(false);
    let out_binaries = if headers_only {
        vec![]
    } else {
        let bin = qpm1
            .info
            .additional_data
            .override_so_name
            .clone()
            .unwrap_or_else(|| format!("lib{}.so", qpm1.info.id));
        vec![PathBuf::from(bin)]
    };

    PackageConfig {
        config_version: package::package_target_version(),
        id: DependencyId(qpm1.info.id.clone()),
        version: qpm1.info.version,
        additional_data: PackageAdditionalData {
            description: String::new(),
            author: String::new(),
            license: String::new(),
            url: qpm1.info.url,
        },
        workspace: WorkspaceConfig {
            scripts: qpm1.workspace.scripts,
            env: None,
            ndk: qpm1.workspace.ndk,
            out_binaries: Some(out_binaries),
            toolchain_out: Some(
                qpm1.info
                    .additional_data
                    .toolchain_out
                    .unwrap_or_else(|| PathBuf::from("toolchain.json")),
            ),
            cmake: qpm1.info.additional_data.cmake,
        },
        qmod: QmodConfig {
            output: qpm1.workspace.qmod_output,
            template: Some(PathBuf::from("mod.template.json")),
            search_dirs: qpm1.workspace.qmod_include_dirs,
            include_files: qpm1.workspace.qmod_include_files,
            download_url: qpm1.info.additional_data.mod_link,
            id: Some(qpm1.info.id),
        },
        dependencies,
        dev_dependencies,
        dependencies_directory: qpm1.dependencies_dir,
        shared_directory: qpm1.shared_dir,
        compile_options: qpm1
            .info
            .additional_data
            .compile_options
            .map(convert_compile_options),
    }
}

fn convert_dependency(dep: &PackageDependencyV1) -> PackageDependency {
    // Both default to true when omitted (see QPM.Package's PackageDependencyModifier).
    let include_qmod = dep.additional_data.include_qmod.unwrap_or(true);
    let required = dep.additional_data.required.unwrap_or(true);

    let qmod = if !include_qmod {
        QmodDependencyMode::None
    } else if !required {
        QmodDependencyMode::Optional
    } else {
        QmodDependencyMode::Required
    };

    PackageDependency {
        version_range: dep.version_range.clone(),
        qmod: Some(qmod),
        qpkg_url: None,
    }
}

fn convert_compile_options(opts: CompileOptionsV1) -> PackageCompileOptions {
    PackageCompileOptions {
        include_paths: opts.include_paths,
        system_includes: opts.system_includes,
        cpp_flags: opts.cpp_flags,
        c_flags: opts.c_flags,
    }
}
