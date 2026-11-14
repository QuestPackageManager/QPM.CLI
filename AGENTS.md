# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

QPM (Quest Package Manager) is a package manager for Quest/Beat Saber C++ mod development. This repo (`QPM.CLI`) builds the `qpm2` binary (package name `qpm_cli`). It depends on two sibling repos, pulled in as git dependencies in `Cargo.toml`:

- **QPM.Package** (`qpm_package` crate) - defines the on-disk config types: `PackageConfig`/`qpm2.json`, `SharedPackageConfig`/`qpm2.shared.json`, `QPkg`/`qpm2.qpkg.json`. If you need to change what a package config *contains*, that's usually in this sibling repo, not here.
- **QPM.Qmod** (`qpm_qmod` crate) - defines `ModJson`, the Beat Saber `mod.json` mod-manifest format that `qpm2 qmod` commands generate.

When working across all three checked out locally, add a `[patch]` block in `Cargo.toml` pointing `qpm_package`/`qpm_qmod` at local paths so `cargo check` picks up unpublished sibling-repo changes - remove it again once those changes are pushed.

Requires the **nightly** Rust toolchain (`rust-toolchain.toml`); edition 2024.

## Commands

```bash
cargo build                              # debug build of the qpm2 binary
cargo build --release                    # what CI ships
cargo check                              # fast type-check, no codegen
cargo check --tests                      # type-check test targets too
cargo test                               # run all test targets
cargo test --test resolve                # mock-repo dependency resolution tests (no network)
cargo test --test network_qpackages      # hits the real qpackages backend; gated by the `network_test` feature (on by default)
cargo test --test commands               # CLI-driving integration tests; currently disabled, see TESTING.md
```

`QPM_DISABLE_GLOBAL_CONFIG=1` should be set when running tests locally (CI sets this) to stop tests from reading/writing the real global `~/.config/QPM-RS2` config.

See `TESTING.md` for the integration-test framework (`tests/commands.rs` + `tests/commands/common.rs`) and how to regenerate fixture expected-output (`QPM_TEST_UPDATE=1`).

## Architecture

### Binary vs. library

Both `src/main.rs` and `src/lib.rs` declare the same top-level modules (`commands`, `models`, `network`, `repository`, `resolver`, `terminal`, `utils`) - the binary doesn't reuse the lib target, it redeclares its own copy of the module tree. Integration tests under `tests/` link against the `qpm_cli` lib target and can only see what's `pub` there.

### Command dispatch

`src/commands/mod.rs` defines `Opt` (the clap `Parser` root), `MainCommand` (the subcommand enum), and the `Command` trait (`fn execute(self) -> Result<()>`). Every subcommand type in `src/commands/*.rs` implements `Command`; `MainCommand::execute` just matches and delegates. `main.rs` calls `Opt::parse()` then `command.execute()`.

### Dependency resolution

`src/resolver/dependency.rs` resolves a package's dependency graph using `pubgrub`. `PackageDependencyResolver` implements pubgrub's `DependencyProvider` with `P = DependencyId` (package identity) and `V = VersionWrapper` (from `resolver/semver.rs`, adapting `semver::Version`/`VersionReq` to pubgrub's range types). `resolve()` does a fresh solve against a `Repository`; `locked_resolve()` replays an already-resolved `SharedPackageConfig` without re-solving. A "resolved dependency" is just a `PackageConfig` (type alias `ResolvedDependency`).

### Repository layering

`Repository` (trait, `src/repository/mod.rs`) is the abstraction for "somewhere packages can be looked up." Implementations compose:
- `FileRepository` (`repository/local.rs`) - the local cache under the user's config dir, plus qpkg install/extract logic.
- `QPMRepository` (`repository/qpackages.rs`) - the remote qpackages.dev backend.
- `MultiDependencyRepository` - merges several repositories (checked in order).
- `MemcachedRepository` - wraps another repository with an in-memory cache for the process's lifetime.

`repository::useful_default_new(offline)` assembles the standard `MemcachedRepository<MultiDependencyRepository>` stack (local cache + remote, or local-only when offline) that most commands use.

### QPKG: build, publish, restore

A `.qpkg` is the distributable artifact format: a zip containing a package's headers (`shared_directory`), its built binaries, and a `qpm2.qpkg.json` manifest (`QPkg { config: PackageConfig, shared_dir, files }`, from `qpm_package::models::qpkg`). This is distinct from `qpm2.json`/`qpm2.shared.json`, which describe a *consuming* project's own dependency graph, not a publishable artifact.

- **Build**: `qpm2 qpkg` (`commands/qpkg.rs`) runs `qpm2 build` first by default (pass `--no-build` to skip and package already-built binaries), then zips `shared_directory` + each `out_binaries` entry (read from the build output dir) + a generated `qpm2.qpkg.json` into `{id}.qpkg`.
- **Install into the local cache**: `FileRepository::install_qpkg` (`repository/local.rs`) extracts a `.qpkg` (from any `Read + Seek` source), splits headers into `{cache}/{id}/{version}/src` and binaries into `.../lib`, verifies every `out_binaries` entry landed on disk, and registers the package into the global cache index (`qpm.repository.json`) via `add_artifact_and_cache`. Two callers:
  - `qpm2 install --path <file>` / `--url <url>` (`commands/install.rs`) - manually side-load a `.qpkg` into the cache.
  - `QPMRepository::download_to_cache` (`repository/qpackages.rs`) - the normal restore path for a remote dependency: fetches a `QPackagesPackage` (`PackageConfig` + `qpkg_url` + `qpkg_checksum`) from the qpackages.dev backend, downloads the `.qpkg` from `qpkg_url`, verifies its SHA-256 against `qpkg_checksum`, then calls `install_qpkg`.
- **Publish**: `qpm2 publish <qpkg_url>` (`commands/publish.rs`) does not upload a `.qpkg` anywhere - the maintainer hosts it themselves (e.g. a GitHub release asset) and this just registers the URL with the backend. It: validates every dependency in `qpm2.shared.json` is still resolvable, downloads the `.qpkg` at the given URL and checks its embedded `PackageConfig` matches the local `qpm2.json` exactly, computes its SHA-256, then POSTs a `QPackagesPackage` to the qpackages.dev API using `--token` or the OS-keyring-stored publish key (`qpm2 config publish`).
- **Legacy path**: `qpm2 install --legacy` bypasses QPKG entirely, directly registering+copying the current project's own already-built binaries into the cache from the working directory (`FileRepository::copy_to_cache`, deprecated in favor of build → `qpm2 qpkg` → `qpm2 install`).

### Config loading and caching

`src/models/config.rs`'s `get_combine_config()` merges the global (`~/.config/QPM-RS2/qpm.settings.json`) and per-workspace (`./qpm.settings.json`) `UserConfig`, and caches the result in a process-wide `OnceLock`. This is correct for a real one-shot `qpm2` invocation (config can't change mid-command) but means an in-process test harness that runs multiple "invocations" in one process only ever sees the first one's config - later invocations silently reuse it. `QPM_DISABLE_GLOBAL_CONFIG=1` short-circuits the global half to a default during tests.

### Cache layout on disk

Package cache paths are built via `PackageIdPath` → `PackageVersionPath` (`src/models/package_files.rs`): `{cache}/{id}/{version}/{src,lib,tmp}`. `FileRepository::collect_deps`/`collect_files_of_package` use these to assemble what gets symlinked/copied into a project's `extern/` directory during restore.

### Integration tests structure

`tests/` are real Cargo integration tests, linking only against `qpm_cli`'s public API:
- `tests/resolve.rs` + `tests/mocks/` - dependency-resolution tests against an in-memory mock `FileRepository`.
- `tests/network_qpackages.rs` - hits the live backend; `#![cfg(feature = "network_test")]`.
- `tests/commands.rs` - the entry point for CLI-driving tests. Since a `tests/<name>.rs` file is itself a crate root, its `mod` declarations can't use the usual "sibling directory named after the file" resolution (that only applies to non-root modules) - they use explicit `#[path = "commands/....rs"]` attributes to pull in files from `tests/commands/`. `tests/commands/common.rs` runs the CLI in-process (parses argv via `Opt::try_parse_from` + calls `.execute()`, not spawning the built binary) against a temp dir seeded from `test_cmd/<name>.in/`, then diffs the result against `test_cmd/<name>.out/`. Because this mutates process-global state (cwd, env vars) across parallel test threads, every test in this target serializes on a single `Mutex`. Currently disabled (see TESTING.md) - its fixtures still use the old QPM v1 JSON format, not the current `qpm2.json`.
