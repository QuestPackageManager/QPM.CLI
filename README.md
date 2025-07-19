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

## Improvements over [Qpm v1](https://github.com/RedBrumbler/QuestPackageManager-Rust)

- `qpm version update` Updates qpm to a newer version
- `qpm s/scripts build` Script system similar to NPM for configuring workspaces
- `qpm ndk download/list/available` NDK management tooling
- `qpm download cmake/ninja` Simplified build tool setup
- `qpm doctor` Configuration diagnostics
- `qpm templatr` Built-in templating

## Quick Setup Commands

```bash
# Change QPM cache location (useful for dev drives)
qpm config cache path

# Set up NDK
qpm ndk resolve -d  # Auto-downloads and configures NDK
```

## NDK Management

### Essential NDK Commands

```bash
# Check NDK configuration
qpm doctor

# List available NDK versions
qpm ndk available

# Download specific NDK version
qpm ndk download 26

# Pin project to NDK version
qpm ndk pin 26 --online

# List installed NDKs
qpm ndk list

# Auto-resolve NDK requirements
qpm ndk resolve -d  # -d flag downloads if needed
```

### NDK Path Configuration

QPM locates the NDK using (in priority order):
1. Project's `ndkpath.txt` file
2. Environment variables (`ANDROID_NDK_HOME` or `ANDROID_NDK_LATEST_HOME`)

```bash
# Set custom NDK path
qpm config ndk-path <your-preferred-path>

# Use existing NDK (e.g., from Android Studio)
qpm config ndk-path <path-to-existing-ndk>
```

### NDK Project Configuration

Specify NDK requirements in `qpm.json`:
```json
"workspace": {
  "ndk": "^26.0.0"
}
```

### Generated CMake Files

QPM generates these files during dependency resolution:
- `extern.cmake`: Configures dependencies
- `qpm_defines.cmake`: Sets up NDK paths and build variables

Include in your CMakeLists.txt:
```cmake
include(${CMAKE_CURRENT_LIST_DIR}/qpm_defines.cmake)
include(${CMAKE_CURRENT_LIST_DIR}/extern.cmake)
```

### Troubleshooting NDK Issues

If NDK paths aren't working:
1. Run `qpm doctor` to check configuration
2. Use `qpm ndk list` to verify installed NDKs
3. Run `qpm ndk resolve -d` to automatically fix issues

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



