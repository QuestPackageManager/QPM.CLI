# QPM (Quest Package Manager)

QPM is a package manager designed specifically for Quest/Beat Saber mods development. It streamlines the process of creating, sharing, and including dependencies in C++ projects.

## Core Features

- **Package Creation & Management**: Create packages and manage dependencies between mods
- **Version Control**: Handle package versions and their dependencies
- **Build Integration**: Seamlessly integrate with build systems
- **NDK Management**: Download and configure Android NDK for Quest development

- `qpm version update` Updates qpm to a newer version
- `qpm s/scripts build` Script system similar to NPM for configuring workspaces
- `qpm ndk download/list/available` NDK management tooling
- `qpm download cmake/ninja` Simplified build tool setup
- `qpm doctor` Configuration diagnostics
- `qpm templatr` Built-in templating

## Workflow

The typical order for taking a mod from a fresh checkout to a published dependency:

```bash
# 1. Resolve and pull in dependencies (headers to extern/includes, binaries to extern/libs)
#    Also writes qpm2.shared.json (the dependency lock file) and, if configured, toolchain.json
qpm restore

# 2. Build the mod's binary using your configured build script (e.g. CMake/ninja)
qpm build
```

At this point you have a compiled binary. What you do with it depends on whether you're shipping
a Quest mod for end users, making the package available as a dependency for other mods, or both.

### Packaging a `.qmod` (for end users / a mod loader)

```bash
# Zip the generated manifest + binaries + any include dirs/files into a sideloadable .qmod
qpm qmod zip
```

### Publishing a `.qpkg` (for other mods to depend on)

A `.qpkg` bundles a package's headers and binaries for other projects to pull in as a dependency.
QPM doesn't host `.qpkg` files itself - you build one, host it yourself (e.g. a GitHub release
asset), then tell the qpackages.dev registry where to find it:

```bash
# Builds by default, then zips headers + binaries + qpm2.qpkg.json into {id}.qpkg
# Pass --no-build to skip the build step and package already-built binaries
qpm qpkg

# Upload/host the resulting .qpkg somewhere publicly accessible, then register it:
qpm publish <url-to-the-hosted-.qpkg> --token <publish-token>
```

`qpm publish` re-downloads the `.qpkg` from the URL you give it to validate its contents match
your local `qpm2.json` before registering it, so the URL must already be live when you run it.

### Installing a `.qpkg` locally (testing before publishing)

Before publishing, you can install the `.qpkg` you just built straight into your local package
cache, so another project's `qpm dependency add`/`qpm restore` will pick it up without needing
anything registered on qpackages.dev:

```bash
# Install a .qpkg file from disk into the local cache
qpm install --path ./{id}.qpkg

# Or install one already hosted somewhere, without registering it on qpackages.dev
qpm install --url <url-to-a-.qpkg>
```

This is the same install step a consumer's `qpm restore` runs automatically for registry
dependencies - `qpm install` just lets you (or someone you hand a `.qpkg` to directly) do it
manually for a package that isn't published, or before you're ready to publish it.

## Configuration & Setup

### Quick Setup Commands

```bash
# Change QPM cache location (useful for dev drives)
qpm config cache path

# Set up NDK
qpm ndk resolve -d  # Auto-downloads and configures NDK
```

## Build & QPKG System

### Understanding QPKG Files

A **`.qpkg`** is the distributable package format - a ZIP containing:
- **Headers** (`sharedDirectory`) - C++ headers for dependents to compile against
- **Binaries** (`workspace.outBinaries`) - Pre-built shared objects (.so files)
- **Manifest** (`qpm2.qpkg.json`) - Package metadata and configuration

When another project runs `qpm restore`, it downloads and extracts your `.qpkg` into:
```
cache/{package-id}/{version}/
├── src/          # Headers extracted here
│   ├── shared/
│   └── qpm2.qpkg.json
└── lib/          # Binaries (.so files) extracted here
```

### Build Integration: `qpm2 build`

The `qpm2 build` command:
1. **Resolves dependencies** - Runs `qpm restore` if needed
2. **Configures toolchain** - Generates CMake integration files
3. **Executes your build script** - Runs the configured build command

#### Configuring Build in `qpm2.json`

Required and optional fields for building:

```json
{
  "configVersion": "2.0.0",
  "id": "my-mod",
  "version": "1.0.0",
  
  "additionalData": {
    "description": "My awesome mod",
    "author": "Your Name",
    "license": "MIT",
    "url": "https://github.com/user/my-mod"
  },
  
  "workspace": {
    "scripts": {
      "build": ["cmake --build build --config Release"]
    },
    "outBinaries": [
      "build/libmymod.so",
      "build/libmymod_debug.so"
    ],
    "ndk": "^26.0.0",
    "toolchainOut": "toolchain.json",
    "cmake": true
  },
  
  "sharedDirectory": "shared",
  "dependenciesDirectory": "extern",
  
  "compileOptions": {
    "cFlags": ["-Wall", "-Werror"],
    "cppFlags": ["-std=c++20"],
    "includePaths": ["shared/include"],
    "systemIncludes": []
  },
  
  "dependencies": {},
  "devDependencies": {}
}
```

#### Build Script Configuration

Configure build scripts in `workspace.scripts` section of `qpm2.json`:

```json
{
  "workspace": {
    "scripts": {
      "build": ["cmake --build build --config Release"],
      "clean": ["rm -rf build"],
      "debug": ["cmake --build build --config Debug"]
    }
  }
}
```

Or add scripts via CLI:

```bash
# Add a build script
qpm scripts build -- cmake --build build --config Release

# Run the build
qpm build
```

### Required Fields for Publishing QPKG

To publish a package, your `qpm2.json` must have these minimum fields:

```json
{
  "configVersion": "2.0.0",
  "id": "package-identifier",          // Unique identifier (lowercase, no spaces)
  "version": "1.2.3",                  // Semantic version (major.minor.patch)
  
  "additionalData": {
    "author": "Your Name"              // Package author
  },
  
  "workspace": {
    "outBinaries": [                   // Binaries to include in QPKG
      "build/libmymod.so"
    ]
  },
  
  "sharedDirectory": "shared",         // Header directory (C++ includes)
  "dependenciesDirectory": "extern",   // Dependencies install location
  
  "qmod": {
    "downloadUrl": "https://github.com/user/my-mod/releases/download/v1.2.3/my-mod.qpkg"
  },
  
  "dependencies": {},
  "devDependencies": {}
}
```

**Validation during publish**:
- SHA-256 checksum is calculated and stored
- Registry verifies your local config matches the uploaded `.qpkg`
- Dependencies must be resolvable from qpackages.dev
- All `workspace.outBinaries` must exist on disk before publishing

## NDK Management

The Android NDK (Native Development Kit) is required for building native C++ code for Quest. QPM automates NDK management, including downloads, path configuration, and CMake integration.

### Quick NDK Setup

```bash
# Auto-detect and download NDK matching project requirements
qpm ndk resolve -d

# Or manually download a specific version
qpm ndk download 26

# Verify NDK is configured correctly
qpm doctor
```

### Specifying NDK Requirements

Define the NDK version your project needs in `qpm2.json`:

```json
{
  "workspace": {
    "ndk": "^26.0.0"
  }
}
```

**Version Specification**:
- `^26.0.0` - Compatible with 26.x.x (recommended)
- `~26.1.0` - Compatible with 26.1.x
- `26` or `26.*` - Any 26.x version
- `=26.2.0` - Exact version only

### Managing NDK Installations

```bash
# List available versions on Google's servers
qpm ndk available

# List locally installed NDKs
qpm ndk list

# Download a specific NDK version
qpm ndk download 26

# Pin project to current installed NDK
qpm ndk pin 26 --online
```

### NDK Path Configuration

QPM locates the NDK using this priority order:

1. **Project-specific path** - `ndkpath.txt` file in project root
2. **Environment variables** - `ANDROID_NDK_HOME` or `ANDROID_NDK_LATEST_HOME`
3. **QPM managed path** - Automatically downloaded by `qpm ndk download`

Configure a custom NDK path:

```bash
# Set global NDK path
qpm config ndk-path <your-ndk-path>

# Use existing NDK (e.g., from Android Studio)
qpm config ndk-path ~/Android/sdk/ndk/26.2.0
```

## Toolchain Generation

QPM can generate a toolchain configuration file with all NDK and build settings. This is useful for:
- Build system configuration
- Cross-compilation setup  
- CI/CD environments
- Reproducible team builds

**Enable toolchain generation** in `qpm2.json`:

```json
{
  "workspace": {
    "toolchainOut": "toolchain.json"
  }
}
```

The toolchain file is automatically generated during `qpm restore` and contains:
- NDK paths and compiler settings
- Architecture and API level configuration
- Dependency paths and link flags
- Compile options from all dependencies

### CI/CD Integration

For CI environments:
```yaml
# Option 1: Set environment variable
env:
  ANDROID_NDK_HOME: /path/to/android-ndk

# Option 2: Automate setup in workflow
- name: Setup NDK
  run: qpm ndk resolve -d
```

## Improvements over [Qpm v1](https://github.com/RedBrumbler/QuestPackageManager-Rust)

- **Enhanced Dependency Resolution** - Pubgrub-based solver with proper conflict detection
- **Modern Config Structure** - Organized workspace and qmod configurations
- **Version Management** - `qpm version update` for automatic QPM updates
- **Script System** - NPM-style scripts for custom build workflows
- **Advanced NDK Tools** - Download, list, and auto-resolve NDK versions
- **Build Tool Setup** - `qpm download cmake/ninja` for simplified setup
- **Diagnostics** - `qpm doctor` for configuration validation
- **Templating** - Built-in `qpm templatr` for project scaffolding



