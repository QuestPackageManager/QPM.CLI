#!/usr/bin/env pwsh

# Build the binary
& cargo build --bin qpm2

# Run all tests with assert_cmd and update fixtures
$ENV:QPM_TEST_UPDATE="1"
& cargo test --test commands -- --nocapture
$ENV:QPM_TEST_UPDATE=""
