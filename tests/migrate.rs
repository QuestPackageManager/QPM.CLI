#![cfg(all(feature = "migrate", feature = "cli"))]

use std::{env, fs, path::PathBuf};

use qpm_cli::commands::{Command, migrate::MigrateCommand};
use qpm_cli::models::package::PackageConfigExtensions;
use qpm_package::models::package::{DependencyId, PackageConfig, QmodDependencyMode};
use semver::Version;
use tempfile::tempdir;

const LEGACY_QPM_JSON: &str = r#"{
  "version": "0.4.0",
  "sharedDir": "shared",
  "dependenciesDir": "extern",
  "info": {
    "name": "MyMod",
    "id": "mymod",
    "version": "1.2.3",
    "url": "https://github.com/example/mymod",
    "additionalData": {
      "cmake": true,
      "modLink": "https://example.com/mymod.qmod",
      "compileOptions": {
        "includePaths": ["include"],
        "cppFlags": ["-std=c++20"]
      }
    }
  },
  "workspace": {
    "scripts": { "build": ["echo hi"] },
    "qmodOutput": "mymod.qmod"
  },
  "dependencies": [
    { "id": "beatsaber-hook", "versionRange": "^6.0.0", "additionalData": { "includeQmod": false } },
    { "id": "some-dev-tool", "versionRange": "^1.0.0", "additionalData": { "private": true, "required": false } }
  ]
}"#;

/// Migrates a legacy qpm.json in a tmp workspace, then reads the resulting qpm2.json back
/// through the same `PackageConfig::read` path every other command uses, proving the output
/// is not just textually plausible but actually round-trips through the real reader.
#[test]
fn migrate_then_read() -> color_eyre::Result<()> {
    let dir = tempdir()?;
    fs::write(dir.path().join("qpm.json"), LEGACY_QPM_JSON)?;

    // MigrateCommand operates relative to the process cwd, like every other Command.
    let previous_dir = env::current_dir()?;
    env::set_current_dir(dir.path())?;
    let migrate_result = MigrateCommand {
        input: None,
        force: false,
    }
    .execute();
    env::set_current_dir(previous_dir)?;
    migrate_result?;

    let package = PackageConfig::read(dir.path())?;

    assert_eq!(package.id, DependencyId("mymod".to_string()));
    assert_eq!(package.version, Version::new(1, 2, 3));
    assert_eq!(package.additional_data.url.as_deref(), Some("https://github.com/example/mymod"));
    assert_eq!(package.workspace.out_binaries, Some(vec!["libmymod.so".into()]));
    assert_eq!(package.workspace.cmake, Some(true));
    assert_eq!(package.workspace.scripts.get("build"), Some(&vec!["echo hi".to_string()]));
    assert_eq!(
        package.qmod.download_url.as_deref(),
        Some("https://example.com/mymod.qmod")
    );
    assert_eq!(package.dependencies_directory, PathBuf::from("extern"));
    assert_eq!(
        package
            .compile_options
            .as_ref()
            .and_then(|c| c.include_paths.clone()),
        Some(vec!["include".to_string()])
    );

    let hook = package
        .dependencies
        .get(&DependencyId("beatsaber-hook".to_string()))
        .expect("beatsaber-hook should be a regular dependency");
    assert_eq!(hook.qmod, Some(QmodDependencyMode::None));

    let dev_tool = package
        .dev_dependencies
        .get(&DependencyId("some-dev-tool".to_string()))
        .expect("private dependencies should become devDependencies");
    assert_eq!(dev_tool.qmod, Some(QmodDependencyMode::Optional));

    Ok(())
}
