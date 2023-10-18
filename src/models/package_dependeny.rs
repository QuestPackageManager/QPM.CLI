use qpm_package::models::{
    dependency::{Dependency, SharedDependency},
    extra::DependencyLibType,
    package::{PackageDependency, PackageConfig},
};

pub trait PackageDependencyExtensions {
    fn infer_lib_type(&self, shared_dep: &PackageConfig) -> DependencyLibType;
}

impl PackageDependencyExtensions for PackageDependency {
    fn infer_lib_type(&self, package: &PackageConfig) -> DependencyLibType {
        let data = self.additional_data.clone();

        let inferred = if package.info.additional_data.static_linking.is_some()
            || package.info.additional_data.static_link.is_some()
        {
            DependencyLibType::Static
        } else {
            DependencyLibType::Shared
        };

        data.lib_type.unwrap_or(inferred)
    }
}
