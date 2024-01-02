use clap::{Args, Subcommand};
use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};

use crate::{
    commands::Command,
    models::package::PackageConfigExtensions,
    repository::{multi::MultiDependencyRepository, self},
    utils::{
        cmake::{write_define_cmake, write_extern_cmake},
        toggle::Toggle,
    },
};

#[derive(Args, Debug, Clone)]

pub struct EditExtraArgs {
    /// Change the branch name in additional data
    #[clap(long = "branchName")]
    pub branch_name: Option<String>,

    /// Change the headers only bool in additional data, pass enable or disable
    #[clap(long = "headersOnly")]
    pub headers_only: Option<Toggle>,

    /// Make the package be statically linked, 0 for false, 1 for true
    #[clap(long = "staticLinking")]
    pub static_linking: Option<Toggle>,

    /// Provide a so link for downloading the regular .so file
    #[clap(long = "soLink")]
    pub so_link: Option<String>,

    /// Provide a debug so link for downloading the debug .so file
    #[clap(long = "debugSoLink")]
    pub debug_so_link: Option<String>,

    /// Provide an overridden name for the .so file
    #[clap(long = "overrideSoName")]
    pub override_so_name: Option<String>,

    /// Provide a link to the mod
    #[clap(long = "modLink")]
    pub mod_link: Option<String>,

    /// If this package is defined in a repo with more packages in subfolders, this is where you specify the subfolder to be used
    #[clap(long = "subFolder")]
    pub sub_folder: Option<String>,

    /// Additional options for compilation and edits to compilation related files.
    #[clap(subcommand)]
    pub compile_options: Option<EditExtraOptions>,

    #[clap(long, default_value = "false")]
    offline: bool,
}

#[derive(Subcommand, Debug, Clone)]

pub enum EditExtraOptions {
    /// Additional options for compilation and edits to compilation related files.
    CompileOptions(CompileOptionsEditArgs),
}

#[derive(Args, Debug, Clone)]

pub struct CompileOptionsEditArgs {
    /// Additional include paths to add, relative to the extern directory. Prefix with a '-' to remove that entry
    #[clap(long = "includePaths")]
    pub include_paths: Option<String>,
    /// Additional system include paths to add, relative to the extern directory. Prefix with a '-' to remove that entry
    #[clap(long = "systemIncludes")]
    pub system_includes: Option<String>,
    /// Additional C++ features to add. Prefix with a '-' to remove that entry
    #[clap(long = "cppFeatures")]
    pub cpp_features: Option<String>,
    /// Additional C++ flags to add. Prefix with a '-' to remove that entry
    #[clap(long = "cppFlags")]
    pub cpp_flags: Option<String>,
    /// Additional C flags to add. Prefix with a '-' to remove that entry
    #[clap(long = "cFlags")]
    pub c_flags: Option<String>,
}

impl Command for EditExtraArgs {
    fn execute(self) -> color_eyre::Result<()> {
        let mut package = PackageConfig::read(".")?;
        let mut any_changed = false;
        if let Some(branch_name) = self.branch_name {
            package_edit_extra_branch_name(&mut package, branch_name);
            any_changed = true;
        }
        if let Some(headers_only) = self.headers_only {
            package_edit_extra_headers_only(&mut package, headers_only.into());
            any_changed = true;
        }
        if let Some(static_linking) = self.static_linking {
            package_edit_extra_static_linking(&mut package, static_linking.into());
            any_changed = true;
        }
        if let Some(so_link) = self.so_link {
            package_edit_extra_so_link(&mut package, so_link);
            any_changed = true;
        }
        if let Some(debug_so_link) = self.debug_so_link {
            package_edit_extra_debug_so_link(&mut package, debug_so_link);
            any_changed = true;
        }
        if let Some(mod_link) = self.mod_link {
            package_edit_extra_mod_link(&mut package, mod_link);
            any_changed = true;
        }
        if let Some(override_so_name) = self.override_so_name {
            package_edit_extra_override_so_name(&mut package, override_so_name);
            any_changed = true;
        }
        if let Some(sub_folder) = self.sub_folder {
            package_edit_extra_sub_folder(&mut package, sub_folder);
            any_changed = true;
        }

        if any_changed {
            package.write(".")?;
            let mut shared_package = SharedPackageConfig::read(".")?;
            shared_package.config = package;
            shared_package.write(".")?;

            // HACK: Not sure if this is a proper way of doing this but it seems logical
            write_define_cmake(&shared_package)?;
            write_extern_cmake(
                &shared_package,
                &repository::useful_default_new(self.offline)?,
            )?;
        }
        Ok(())
    }
}

pub fn package_edit_extra_branch_name(package: &mut PackageConfig, branch_name: String) {
    println!("Setting branch name: {branch_name:#?}");
    package.info.additional_data.branch_name = Some(branch_name);
}

pub fn package_edit_extra_headers_only(package: &mut PackageConfig, headers_only: bool) {
    println!("Setting headers_only: {headers_only:#?}");
    package.info.additional_data.headers_only = Some(headers_only);
}

pub fn package_edit_extra_static_linking(package: &mut PackageConfig, static_linking: bool) {
    println!("Setting static_linking: {static_linking:#?}");
    package.info.additional_data.static_linking = Some(static_linking);
}

pub fn package_edit_extra_so_link(package: &mut PackageConfig, so_link: String) {
    println!("Setting so_link: {so_link:#?}");
    package.info.additional_data.so_link = Some(so_link);
}

pub fn package_edit_extra_mod_link(package: &mut PackageConfig, mod_link: String) {
    println!("Setting mod_link: {mod_link:#?}");
    package.info.additional_data.mod_link = Some(mod_link);
}

pub fn package_edit_extra_debug_so_link(package: &mut PackageConfig, debug_so_link: String) {
    println!("Setting debug_so_link: {debug_so_link:#?}");
    package.info.additional_data.debug_so_link = Some(debug_so_link);
}

pub fn package_edit_extra_override_so_name(package: &mut PackageConfig, override_so_name: String) {
    println!("Setting override_so_name: {override_so_name:#?}");
    package.info.additional_data.override_so_name = Some(override_so_name);
}

pub fn package_edit_extra_sub_folder(package: &mut PackageConfig, sub_folder: String) {
    println!("Setting sub_folder: {sub_folder:#?}");
    package.info.additional_data.sub_folder = Some(sub_folder);
}
