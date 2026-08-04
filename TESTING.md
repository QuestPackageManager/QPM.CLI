# Testing in QPM.CLI

This project uses a modern test framework built on top of `assert_cmd`, `assert_fs`, `fs_extra`, and other testing libraries to test CLI commands.

## Test Directory Structure

Each test case follows this directory structure:

```
test_cmd/
  ├── test_name.in/   # Input files for the test
  │     └── ...
  └── test_name.out/  # Expected output files
        └── ...
```

For example, the dependency add test has:

```
test_cmd/
  ├── dep_add.in/     # Input files for dependency add
  │     └── qpm.json
  └── dep_add.out/    # Expected output after adding dependency
        └── qpm.json
```

These tests drive the built `qpm` binary via `assert_cmd::Command::cargo_bin`, which requires the `CARGO_BIN_EXE_qpm` env var that Cargo only sets for integration test targets - so this suite lives under `tests/`, not as a unit test inside `src/`.

## Running Tests

To run all tests:

```bash
cargo test --test commands
```

To update test fixtures:

```bash
QPM_TEST_UPDATE=1 cargo test --test commands
```

## Writing New Tests

To create a new test:

1. Create input and output directories:
   ```
   test_cmd/my_test.in/
   test_cmd/my_test.out/
   ```

2. Add the test files to the input directory

3. Add a test function in `tests/commands.rs` using the `test_command` function:

   ```rust
   #[test]
   fn test_my_feature() -> color_eyre::Result<()> {
       common::test_command(
           &["my", "command", "--arg"],
           Path::new("test_cmd/my_test.in"),
           Path::new("test_cmd/my_test.out"),
       )
   }
   ```

4. Run the test with the `QPM_TEST_UPDATE` environment variable to generate the expected output files:
   ```
   QPM_TEST_UPDATE=1 cargo test --test commands my_feature
   ```

## Test Framework

The test framework is in `tests/commands/` and consists of:

- `common.rs`: Test utilities for running commands and comparing directories

The framework provides functions for:

- `test_command`: Runs a command and compares the output directory with the expected directory
- `test_command_check_files`: Runs a command and checks that specific files exist
- `assert_directory_equal`: Compares two directories recursively

All error handling uses `color_eyre` for better error reporting and debugging.
