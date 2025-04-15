use serde::Serialize;

/// A struct that wraps a value with a schema reference.
#[derive(Serialize)]
pub struct WithSchema<'a, T> {
    /// The schema reference.
    #[serde(rename = "$schema")]
    pub schema: &'a str,

    /// The wrapped value.
    #[serde(flatten)]
    pub value: T,
}

pub struct SchemaLinks;

impl SchemaLinks {
    pub const PACKAGE_CONFIG: &'static str = "https://raw.githubusercontent.com/QuestPackageManager/QPM.Package/refs/heads/main/qpm.schema.json";
    pub const SHARED_PACKAGE_CONFIG: &'static str = "https://raw.githubusercontent.com/QuestPackageManager/QPM.Package/refs/heads/main/qpm.shared.schema.json";
    pub const USER_CONFIG: &'static str = "https://raw.githubusercontent.com/QuestPackageManager/QPM.CLI/refs/heads/main/qpm.settings.schema.json";
    pub const FILE_REPOSITORY: &'static str = "https://raw.githubusercontent.com/QuestPackageManager/QPM.CLI/refs/heads/main/qpm.repository.schema.json";
    pub const TOOLCHAIN_DATA: &'static str = "https://raw.githubusercontent.com/QuestPackageManager/QPM.CLI/refs/heads/main/qpm.toolchain.schema.json";
    pub const MOD_CONFIG: &'static str = "https://raw.githubusercontent.com/Lauriethefish/QuestPatcher.QMod/refs/heads/main/QuestPatcher.QMod/Resources/qmod.schema.json";
}
