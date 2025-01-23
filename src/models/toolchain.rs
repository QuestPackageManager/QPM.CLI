use std::{fs::File, path::PathBuf};

use color_eyre::eyre::Result;
use qpm_package::models::{dependency::SharedPackageConfig, extra::CompileOptions};
use serde::{Deserialize, Serialize};

use crate::repository::Repository;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ToolchainData {
    pub compile_options: CompileOptions,
    /// relative to package dir
    pub extern_dir: PathBuf,

    pub binary_out: Option<PathBuf>,
    pub debug_binary_out: Option<PathBuf>,
}

pub fn write_toolchain_file(
    shared_config: &SharedPackageConfig,
    repo: &impl Repository,
    toolchain_path: &std::path::PathBuf,
) -> Result<()> {
    let extern_dir = &shared_config.config.dependencies_dir.display();
    let compile_options = shared_config
        .restored_dependencies
        .iter()
        .filter_map(|s| {
            let shared_config = repo.get_package(&s.dependency.id, &s.version).ok()??;

            let package_id = &shared_config.config.info.id;

            let prepend_path = |dir: &String| format!("{extern_dir}/includes/{package_id}/{dir}");

            let mut compile_options = shared_config.config.info.additional_data.compile_options?;

            // prepend path
            compile_options.include_paths = compile_options
                .include_paths
                .map(|v| v.iter().map(prepend_path).collect());
            compile_options.system_includes = compile_options
                .system_includes
                .map(|v| v.iter().map(prepend_path).collect());

            Some(compile_options)
        })
        .fold(CompileOptions::default(), |acc, x| {
            let c_flags: Vec<String> = acc
                .c_flags
                .unwrap_or_default()
                .into_iter()
                .chain(x.c_flags.unwrap_or_default())
                .collect();
            let cpp_flags: Vec<String> = acc
                .cpp_flags
                .unwrap_or_default()
                .into_iter()
                .chain(x.cpp_flags.unwrap_or_default())
                .collect();
            let include_paths: Vec<String> = acc
                .include_paths
                .unwrap_or_default()
                .into_iter()
                .chain(x.include_paths.unwrap_or_default())
                .collect();

            let system_includes: Vec<String> = acc
                .system_includes
                .unwrap_or_default()
                .into_iter()
                .chain(x.system_includes.unwrap_or_default())
                .collect();

            let cpp_features: Vec<String> = acc
                .cpp_features
                .unwrap_or_default()
                .into_iter()
                .chain(x.cpp_features.unwrap_or_default())
                .collect();

            CompileOptions {
                c_flags: Some(c_flags),
                cpp_flags: Some(cpp_flags),
                include_paths: Some(include_paths),
                system_includes: Some(system_includes),
                cpp_features: Some(cpp_features),
            }
        });

    let toolchain = ToolchainData {
        compile_options,
        extern_dir: shared_config.config.dependencies_dir.clone(),
        // TODO:
        binary_out: None,
        debug_binary_out: None,
    };
    let file = File::create(toolchain_path)?;
    serde_json::to_writer_pretty(file, &toolchain)?;
    Ok(())
}
