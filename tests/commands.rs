// Tests for CLI commands: each runs the qpm2 CLI in-process against a fixture in test_cmd/.
// Regenerate expected outputs with QPM_TEST_UPDATE=1.
#![cfg(all(feature = "cli", feature = "network_test"))]

#[path = "commands/common.rs"]
mod common;

/// Tests for the dependency command
#[path = "commands/dependency.rs"]
mod dependency;

// Still disabled: their fixtures are in the QPM v1 format (qpm.json) and they depend on
// packages the qpm2 backend doesn't serve (beatsaber-hook) or on large downloads (NDK, adb).

// /// This module contains the tests for the ndk command
// #[path = "commands/ndk.rs"]
// mod ndk;

// /// This module contains the tests for the qmod command
// #[path = "commands/qmod.rs"]
// mod qmod;

// /// This module contains the tests for the restore command
// #[path = "commands/restore.rs"]
// mod restore;

// /// This module contains the tests for the download command
// #[path = "commands/download.rs"]
// mod download;
