use qpm_package::models::{
    dependency::{Dependency, SharedDependency},
    extra::DependencyLibType,
    package::{PackageConfig, PackageDependency},
};

pub trait PackageDependencyExtensions {
    fn infer_lib_type(&self, package: &PackageConfig) -> DependencyLibType;

    fn use_as_header_only(&self, package: &PackageConfig) -> bool;
    fn use_as_static(&self, package: &PackageConfig) -> bool;
    fn use_as_shared(&self, package: &PackageConfig) -> bool;
}

impl PackageDependencyExtensions for PackageDependency {
    fn infer_lib_type(&self, package: &PackageConfig) -> DependencyLibType {
        let data = self.additional_data.clone();

        let inferred = if package.info.additional_data.static_linking.is_some()
            || package.info.additional_data.static_link.is_some()
        {
            DependencyLibType::Static
        } else if package.info.additional_data.headers_only == Some(true) {
            DependencyLibType::HeaderOnly
        } else {
            DependencyLibType::Shared
        };

        data.lib_type.unwrap_or(inferred)
    }

    fn use_as_header_only(&self, package: &PackageConfig) -> bool {
        self.infer_lib_type(package) == DependencyLibType::HeaderOnly
    }

    fn use_as_static(&self, package: &PackageConfig) -> bool {
        self.infer_lib_type(package) == DependencyLibType::Static
    }

    fn use_as_shared(&self, package: &PackageConfig) -> bool {
        self.infer_lib_type(package) == DependencyLibType::Shared
    }
}
