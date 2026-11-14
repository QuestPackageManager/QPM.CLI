# QPM v2 vs QPM v1

"QPM v1" here means this repository's `main` branch (package `qpm_cli` v0.1.0, binary name
`qpm`, config file `qpm.json`) — not the older
[RedBrumbler/QuestPackageManager-Rust](https://github.com/RedBrumbler/QuestPackageManager-Rust)
project that `main`'s own README separately compares itself against. "QPM v2" is this branch
(`qpm2`): package `qpm_cli` v2.0.0, binary name `qpm2`, config file `qpm2.json`.

A lot of the CLI surface — `restore`, `dependency`, `ndk`, `scripts`, `doctor`, `templatr`,
`version update`, `genschema`, the pubgrub-based resolver — already exists on `main`. This
document only covers what actually changed.

## 1. QPKG: a real distributable package artifact

v1 has no packaging step. `qpm install <binary_path> <debug_binary_path>` takes prebuilt binary
file paths straight off the CLI and copies them into the cache from whatever's in the working
directory — headers and binaries aren't bundled together, and there's no manifest or checksum
tying them to a specific config.

v2 introduces `.qpkg`: a zip containing a package's headers (`sharedDirectory`), its built
binaries (`workspace.outBinaries`), and a generated `qpm2.qpkg.json` manifest. `qpm2 qpkg` builds
one (running `qpm2 build` first by default); `qpm2 install --path/--url` installs one into the
local cache, verifying every declared binary actually landed on disk. The local cache itself now
splits headers into `{id}/{version}/src` and binaries into `{id}/{version}/lib` instead of one
flat blob per package (`src/repository/file.rs` vs v1's `src/repository/local.rs`).

## 2. `qpm2 build` — a first-class build command

v1 has no `build` command; you ran your own build tool (cmake/ninja/etc.) yourself, then handed
the resulting binary to `qpm install`. v2 adds `qpm2 build`, which resolves dependencies,
generates the toolchain integration files, and runs your configured build script as one step —
and `qpm2 qpkg` runs it automatically before packaging.

## 3. Stronger publish integrity guarantees

v1's `qpm publish` checks that your dependencies are resolvable, then registers your package's
metadata as-is — it trusts that `info.url` already points at binaries you host and manage
yourself, with no verification step.

v2's `qpm2 publish <qpkg-url>` re-downloads the `.qpkg` at the URL you give it, checks that its
embedded `PackageConfig` matches your local `qpm2.json` *exactly*, computes its SHA-256, and only
then registers `{PackageConfig, qpkg_url, qpkg_checksum}` with the backend. A consumer's
`qpm2 restore` later verifies that same checksum against whatever it downloads, so a corrupted or
tampered `.qpkg` is caught before it's linked into someone's build (`src/services/publish.rs`).

## 4. Config format rewrite (`qpm.json` → `qpm2.json`)

The package config was restructured, not just renamed:

| | v1 (`qpm.json`) | v2 (`qpm2.json`) |
|---|---|---|
| Identity/metadata | nested under `info: { id, version, author, url, additionalData }` | flattened to top-level `id`/`version`, metadata moved to `additionalData` |
| Dependencies | `Array<{ id, versionRange, additionalData }>` | `Record<id, { ... }>` map, split into `dependencies` / `devDependencies` |
| Build config | scattered under `info.additionalData` and `workspace` | consolidated under `workspace` (`scripts`, `env`, `ndk`, `outBinaries`, `toolchainOut`, `cmake`) |
| Qmod config | `workspace.qmodIncludeDirs`/`qmodIncludeFiles`/`qmodOutput` | dedicated `qmod` block (`output`, `template`, `searchDirs`, `includeFiles`, `downloadUrl`, `id`) |
| Versioning | no schema version field | `configVersion` field |

A `migrate.ts` script ships in this repo to convert an existing v1 `qpm.json`/`qpm.shared.json`
project to the v2 format automatically.

## 5. Repository / service layer rework

v1's business logic was split across `src/utils/{cmake,git,ndk}.rs`, `src/network/*`, and
`src/resolver/dependency.rs`. v2 consolidates this into `src/services/*`
(`android`, `git`, `github`, `ndk`, `network`, `pubgrub`, `publish`, `qpm_version`, `restore`),
and separates "solve the dependency graph" (`services/pubgrub.rs`) from "actually download and
lay out the result on disk" (`services/restore.rs`), which used to be one path in v1's resolver.

## 6. Trimmed dependency footprint

v1's default feature set includes `gitoxide`, which pulls in the `gix` crate (and, through it, a
`reqwest`-based HTTP client) purely so `templatr` can clone template repos without shelling out to
`git`. v2 drops `gitoxide` from defaults entirely — `templatr` now uses its `git_cli` backend
(shells out to the system `git` binary) instead — removing that whole dependency subtree. Despite
v2 adding the `qpkg`/`build` commands and more test coverage, `Cargo.lock` is net *smaller* than
v1's.

## 7. Vastly expanded test suite

v1's tests live inline under `src/tests/` (`commands.rs`, `mocks/repo.rs`, `network/qpackages.rs`,
`resolve.rs` — a few hundred lines total). v2 moves to a proper Cargo integration-test target
under `tests/`, with dedicated suites for the new functionality this doc describes:
`qpkg_management.rs`, `file_repository.rs`, `publish.rs`, `repository_multi.rs`, `restore.rs`,
`package_files.rs`, `github.rs`, `git.rs`, `ndk.rs`, `android.rs`, `semver.rs`,
`qpm_version.rs`, plus a `tests/commands/` harness that drives the CLI in-process
(`dependency.rs`, `download.rs`, `ndk.rs`, `qmod.rs`, `restore.rs`).

## 8. Naming and versioning

Binary renamed `qpm` → `qpm2` (so both can be installed side by side during a migration period),
package version bumped `0.1.0` → `2.0.0` to signal the breaking config-format change.
