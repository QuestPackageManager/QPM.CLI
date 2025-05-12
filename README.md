# QPM.CLI

QPM command line tool

# Improvements over [Qpm v1](https://github.com/RedBrumbler/QuestPackageManager-Rust)

- `qpm version update` Updates qpm to a newer version.
- `qpm s/scripts build` Script system similar to NPM for configuring workspaces.
- `qpm ndk download {version}/list/available` Downloading and managing NDK installations
- `qpm download cmake/ninja` for setting up CMake and Ninja.
- `qpm doctor` for checking if everything is setup properly.
- `qpm templatr` embedded.
- Reports progress when downloading or cloning
- Leverages local cache for faster restores (and even offline usage, TODO)
- Rewritten from the ground up to use functional patterrns, immutability and declarative code style. Results in better reliability and consistency.
- ~~Supports locked restore `qpm restore --locked`~~ This is the default now
- More modular
- Easier to maintain
- Could support mirrors or other backends
- Better error handlingg
- Is tested (not thoroughly yet)
