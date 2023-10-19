use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use color_eyre::{eyre::Context, Result};
use qpm_package::{models::dependency::SharedPackageConfig, extensions::package_metadata::PackageMetadataExtensions};

use crate::repository::Repository;
use std::fmt::Write as OtherWrite;

const EXTERN_CMAKE_FILE: &str = "extern.cmake";
const QPM_CMAKE_FILE: &str = "qpm_defines.cmake";

/// Fern: Adds line ending after each element
/// thanks raft
macro_rules! concatln {
    ($s:expr $(, $ss:expr)*) => {
        concat!($s $(, "\n", $ss)*)
    }
}

pub fn write_cmake(shared_package: &SharedPackageConfig, repo: &impl Repository) -> Result<()> {
    let cmake_opt = shared_package.config.info.additional_data.cmake;

    if cmake_opt.is_none() && Path::new("./CMakeLists.txt").exists() {
        eprintln!("qpm.json::info::additional_data::cmake is undefined in a CMake project, consider setting it to true");
    }

    // default to true
    let cmake = cmake_opt.unwrap_or(true);
    if !cmake {
        return Ok(());
    }
    write_extern_cmake(shared_package, repo)?;
    write_define_cmake(shared_package)?;

    Ok(())
}

pub fn write_extern_cmake(dep: &SharedPackageConfig, repo: &impl Repository) -> Result<()> {
    let path = Path::new("./").join(EXTERN_CMAKE_FILE);
    let mut extern_cmake_file =
        File::create(path).context(format!("Unable to create {EXTERN_CMAKE_FILE}"))?;
    let mut result = concatln!(
            "# YOU SHOULD NOT MANUALLY EDIT THIS FILE, QPM WILL VOID ALL CHANGES",
            "# always added",
            "target_include_directories(${COMPILE_ID} PRIVATE ${EXTERN_DIR}/includes)",
            "target_include_directories(${COMPILE_ID} SYSTEM PRIVATE ${EXTERN_DIR}/includes/libil2cpp/il2cpp/libil2cpp)",
            "\n# includes and compile options added by other libraries\n"
        ).to_string();

    let mut any = false;
    for shared_dep in dep.restored_dependencies.iter() {
        let shared_package = repo
            .get_package(&shared_dep.dependency.id, &shared_dep.version)
            .context("Unable to get shared package")?
            .unwrap();
        let package_id = shared_package.config.info.id;

        if let Some(compile_options) = shared_package.config.info.additional_data.compile_options {
            any = true;
            // TODO: Must ${{COMPILE_ID}} be changed to {package_id}?

            if let Some(include_dirs) = compile_options.include_paths {
                for dir in include_dirs.iter() {
                    writeln!(result, "target_include_directories(${{COMPILE_ID}} PRIVATE ${{EXTERN_DIR}}/includes/{package_id}/{dir})")?;
                }
            }

            if let Some(system_include_dirs) = compile_options.system_includes {
                for dir in system_include_dirs.iter() {
                    writeln!(result, "target_include_directories(${{COMPILE_ID}} SYSTEM PRIVATE ${{EXTERN_DIR}}/includes/{package_id}/{dir})")?;
                }
            }

            let mut features: Vec<String> = vec![];

            if let Some(cpp_features) = compile_options.cpp_features {
                features.append(&mut cpp_features.clone());
            }

            for feature in features.iter() {
                writeln!(
                    result,
                    "target_compile_features(${{COMPILE_ID}} PRIVATE {feature})"
                )?;
            }

            let mut flags: Vec<String> = vec![];

            if let Some(cpp_flags) = compile_options.cpp_flags {
                flags.append(&mut cpp_flags.clone());
            }

            if let Some(c_flags) = compile_options.c_flags {
                flags.append(&mut c_flags.clone());
            }

            for flag in flags.iter() {
                writeln!(
                    result,
                    "target_compile_options(${{COMPILE_ID}} PRIVATE {flag})"
                )?;
            }
        }

        //TODO: Revisit
        // if let Some(extra_files) = &shared_dep.dependency.additional_data.extra_files {
        //     for path_str in extra_files.iter() {
        //         let path = PathBuf::new().join(&format!(
        //             "extern/includes/{}/{}",
        //             &shared_dep.dependency.id, path_str
        //         ));
        //         let extern_path = PathBuf::new().join(&format!(
        //             "includes/{}/{}",
        //             &shared_dep.dependency.id, path_str
        //         ));
        //         if path.is_file() {
        //             writeln!(
        //                 result,
        //                 "add_library(${{COMPILE_ID}} SHARED ${{EXTERN_DIR}}/{})",
        //                 extern_path.display()
        //             )?;
        //         } else {
        //             let listname = format!(
        //                 "{}_{}_extra",
        //                 path_str.replace(['/', '\\', '-'], "_"),
        //                 shared_dep.dependency.id.replace('-', "_")
        //             );

        //             writeln!(
        //                 result,
        //                 "RECURSE_FILES({}_c ${{EXTERN_DIR}}/{}/*.c)",
        //                 listname,
        //                 extern_path.display()
        //             )?;

        //             writeln!(
        //                 result,
        //                 "RECURSE_FILES({}_cpp ${{EXTERN_DIR}}/{}/*.cpp)",
        //                 listname,
        //                 extern_path.display()
        //             )?;

        //             writeln!(
        //                 result,
        //                 "target_sources(${{COMPILE_ID}} PRIVATE ${{{listname}_c}})"
        //             )?;

        //             writeln!(
        //                 result,
        //                 "target_sources(${{COMPILE_ID}} PRIVATE ${{{listname}_cpp}})"
        //             )?;
        //         }
        //     }
        // }

        if let Some(dep) = dep
            .config
            .dependencies
            .iter()
            .find(|el| el.id == shared_dep.dependency.id)
        {
            if let Some(extra_files) = &dep.additional_data.extra_files {
                for path_str in extra_files.iter() {
                    let path =
                        PathBuf::new().join(&format!("extern/includes/{}/{}", &dep.id, path_str));
                    let extern_path = std::path::PathBuf::new().join(&format!(
                        "includes/{}/{}",
                        &shared_dep.dependency.id, path_str
                    ));
                    if path.is_file() {
                        write!(
                            result,
                            "add_library(${{COMPILE_ID}} SHARED ${{EXTERN_DIR}}/{})",
                            extern_path.display()
                        )?;
                    } else {
                        let listname = format!(
                            "{}_{}_local_extra",
                            path_str.replace(['/', '\\', '-'], "_"),
                            shared_dep.dependency.id.replace('-', "_")
                        );

                        writeln!(
                            result,
                            "RECURSE_FILES({}_c ${{EXTERN_DIR}}/{}/*.c)",
                            listname,
                            extern_path.display()
                        )?;

                        writeln!(
                            result,
                            "RECURSE_FILES({}_cpp ${{EXTERN_DIR}}/{}/*.cpp)",
                            listname,
                            extern_path.display()
                        )?;

                        writeln!(
                            result,
                            "target_sources(${{COMPILE_ID}} PRIVATE ${{{listname}_c}})"
                        )?;

                        writeln!(
                            result,
                            "target_sources(${{COMPILE_ID}} PRIVATE ${{{listname}_cpp}})"
                        )?;
                    }
                }
            }
        }
    }

    if !any {
        result.push_str("# Sadly, there were none with extra include dirs\n");
    }

    result.push_str(concatln!(
        "\n# libs dir -> stores .so or .a files (or symlinked!)",
        "target_link_directories(${COMPILE_ID} PRIVATE ${EXTERN_DIR}/libs)",
        "RECURSE_FILES(so_list ${EXTERN_DIR}/libs/*.so)",
        "RECURSE_FILES(a_list ${EXTERN_DIR}/libs/*.a)\n",
        "# every .so or .a that needs to be linked, put here!",
        "# I don't believe you need to specify if a lib is static or not, poggers!",
        "target_link_libraries(${COMPILE_ID} PRIVATE\n\t${so_list}\n\t${a_list}\n)\n"
    ));

    extern_cmake_file
        .write_all(result.as_bytes())
        .context("Failed to write out extern cmake file")?;
    Ok(())
}

pub fn write_define_cmake(dep: &SharedPackageConfig) -> Result<()> {
    let path = Path::new("./").join(QPM_CMAKE_FILE);

    let mut defines_cmake_file =
        File::create(path).context("Failed to create defines cmake file")?;

    defines_cmake_file
        .write_all(make_defines_string(dep)?.as_bytes())
        .context("Failed to write out own define make string")?;
    Ok(())
}

pub fn make_defines_string(dep: &SharedPackageConfig) -> Result<String> {
    // TODO: use additional_data.compile_options here or in the extern cmake file ? include dirs are set there at least
    let mut result: String = concatln!(
        "# YOU SHOULD NOT MANUALLY EDIT THIS FILE, QPM WILL VOID ALL CHANGES",
        "# Version defines, pretty useful"
    )
    .to_string();

    writeln!(result, "\nset(MOD_VERSION \"{}\")", dep.config.info.version)?;
    result.push_str("# take the mod name and just remove spaces, that will be MOD_ID, if you don't like it change it after the include of this file\n");
    writeln!(
        result,
        "set(MOD_ID \"{}\")\n",
        dep.config.info.name.replace(' ', "")
    )?;
    result.push_str("# derived from override .so name or just id_version\n");

    writeln!(
        result,
        "set(COMPILE_ID \"{}\")",
        dep.config.info.get_module_id()
    )?;

    result.push_str(
        "# derived from whichever codegen package is installed, will default to just codegen\n",
    );

    writeln!(
        result,
        "set(CODEGEN_ID \"{}\")\n",
        if let Some(codegen_dep) = dep
            .restored_dependencies
            .iter()
            .find(|dep| dep.dependency.id.contains("codegen"))
        {
            // found a codegen
            &codegen_dep.dependency.id
        } else {
            "codegen"
        }
    )?;

    result.push_str("# given from qpm, automatically updated from qpm.json\n");

    writeln!(
        result,
        "set(EXTERN_DIR_NAME \"{}\")",
        dep.config.dependencies_dir.display()
    )?;
    writeln!(
        result,
        "set(SHARED_DIR_NAME \"{}\")\n",
        dep.config.shared_dir.display()
    )?;

    result.push_str(concatln!(
        "# if no target given, use Debug",
        "if (NOT DEFINED CMAKE_BUILD_TYPE)",
        "\tset(CMAKE_BUILD_TYPE \"Debug\")",
        "endif()\n"
    ));
    result.push_str(concatln!(
        "\n# defines used in ninja / cmake ndk builds",
        "if (NOT DEFINED CMAKE_ANDROID_NDK)",
        "\tif (EXISTS \"${CMAKE_CURRENT_LIST_DIR}/ndkpath.txt\")",
        "\t\tfile (STRINGS \"ndkpath.txt\" CMAKE_ANDROID_NDK)",
        "\telse()",
        "\t\tif(EXISTS $ENV{ANDROID_NDK_HOME})",
        "\t\t\tset(CMAKE_ANDROID_NDK $ENV{ANDROID_NDK_HOME})",
        "\t\telseif(EXISTS $ENV{ANDROID_NDK_LATEST_HOME})",
        "\t\t\tset(CMAKE_ANDROID_NDK $ENV{ANDROID_NDK_LATEST_HOME})",
        "\t\tendif()",
        "\tendif()",
        "endif()",
        "if (NOT DEFINED CMAKE_ANDROID_NDK)",
        "\tmessage(Big time error buddy, no NDK)",
        "endif()",
        "message(Using NDK ${CMAKE_ANDROID_NDK})",
        "string(REPLACE \"\\\\\" \"/\" CMAKE_ANDROID_NDK ${CMAKE_ANDROID_NDK})",
        "\nset(ANDROID_PLATFORM 24)",
        "set(ANDROID_ABI arm64-v8a)",
        "set(ANDROID_STL c++_static)",
        "set(ANDROID_USE_LEGACY_TOOLCHAIN_FILE OFF)",
        "\nset(CMAKE_TOOLCHAIN_FILE ${CMAKE_ANDROID_NDK}/build/cmake/android.toolchain.cmake)"
    ));
    result.push_str(concatln!(
        "\n# define used for external data, mostly just the qpm dependencies",
        "set(EXTERN_DIR ${CMAKE_CURRENT_SOURCE_DIR}/${EXTERN_DIR_NAME})",
        "set(SHARED_DIR ${CMAKE_CURRENT_SOURCE_DIR}/${SHARED_DIR_NAME})"
    ));
    result.push_str(concatln!(
        "\n# get files by filter recursively",
        "MACRO(RECURSE_FILES return_list filter)",
        "\tFILE(GLOB_RECURSE new_list ${filter})",
        "\tSET(file_list \"\")",
        "\tFOREACH(file_path ${new_list})",
        "\t\tSET(file_list ${file_list} ${file_path})",
        "\tENDFOREACH()",
        "\tLIST(REMOVE_DUPLICATES file_list)",
        "\tSET(${return_list} ${file_list})",
        "ENDMACRO()"
    ));

    Ok(result)
}
