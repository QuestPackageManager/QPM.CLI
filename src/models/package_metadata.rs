use std::path::PathBuf;

use qpm_package::models::{dependency::SharedDependency, package::PackageMetadata};

pub trait PackageMetadataExtensions {
    fn get_so_name(&self) -> String;

    fn get_module_id(&self) -> String {
        let name = self.get_so_name();
        PathBuf::new()
            .with_file_name(name[3..name.len() - 2].to_string())
            .with_extension("")
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }
}

impl PackageMetadataExtensions for PackageMetadata {
    fn get_so_name(&self) -> String {
        self.additional_data
            .override_so_name
            .clone()
            .unwrap_or(format!(
                "lib{}_{}.{}",
                self.id,
                self.version.to_string().replace('.', "_"),
                if self.additional_data.static_linking.unwrap_or(false) {
                    "a"
                } else {
                    "so"
                },
            ))
    }
}

impl PackageMetadataExtensions for SharedDependency {
    fn get_so_name(&self) -> String {
        self.dependency
            .additional_data
            .override_so_name
            .clone()
            .unwrap_or(format!(
                "lib{}_{}.{}",
                self.dependency.id,
                self.version.to_string().replace('.', "_"),
                if self
                    .dependency
                    .additional_data
                    .static_linking
                    .unwrap_or(false)
                {
                    "a"
                } else {
                    "so"
                },
            ))
    }
}
