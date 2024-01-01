use std::path::Path;

use color_eyre::eyre::{anyhow, Result};
use qpm_package::models::{
    dependency::Dependency,
    extra::{AdditionalPackageMetadata, DependencyLibType},
    package::PackageDependency,
};

use crate::terminal::colors::QPMColor;

pub trait PackageDependencyExtensions {
    fn infer_lib_type(&self, additional_data: &AdditionalPackageMetadata) -> DependencyLibType;

    fn usable_as_header_only(&self, additional_data: &AdditionalPackageMetadata) -> bool;
    fn usable_as_static(&self, additional_data: &AdditionalPackageMetadata) -> bool;
    fn usable_as_shared(&self, additional_data: &AdditionalPackageMetadata) -> bool;
}

impl PackageDependencyExtensions for PackageDependency {
    fn infer_lib_type(&self, additional_data: &AdditionalPackageMetadata) -> DependencyLibType {
        let data = self.additional_data.clone();

        let inferred = if additional_data.static_link.is_some() {
            DependencyLibType::Static
        } else if additional_data.headers_only == Some(true) {
            DependencyLibType::HeaderOnly
        } else {
            DependencyLibType::Shared
        };

        data.lib_type.unwrap_or(inferred)
    }

    fn usable_as_header_only(&self, additional_data: &AdditionalPackageMetadata) -> bool {
        self.infer_lib_type(additional_data) == DependencyLibType::HeaderOnly
    }

    fn usable_as_static(&self, additional_data: &AdditionalPackageMetadata) -> bool {
        self.infer_lib_type(additional_data) == DependencyLibType::Static
    }

    fn usable_as_shared(&self, additional_data: &AdditionalPackageMetadata) -> bool {
        self.infer_lib_type(additional_data) == DependencyLibType::Shared
    }
}

pub trait DependencyExtensions {
    fn get_static_lib_out(&self) -> Result<&Path>;
    fn get_dynamic_lib_out(&self) -> Result<&Path>;
}

impl DependencyExtensions for Dependency {
    fn get_static_lib_out(&self) -> Result<&Path> {
        let path = self
            .additional_data
            .static_lib_out
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "{} qpm.shared.json::info::additionalData::staticLibOut not defined",
                    self.id.dependency_id_color()
                )
            })?;

        Ok(path)
    }

    fn get_dynamic_lib_out(&self) -> Result<&Path> {
        let path = self
            .additional_data
            .dynamic_lib_out
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "{} qpm.shared.json::info::additionalData::dynamicLibOut not defined",
                    self.id.dependency_id_color()
                )
            })?;

        Ok(path)
    }
}
