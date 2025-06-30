#!/usr/bin/env pwsh

# Build the binary
& cargo build --bin qpm 

# Run all tests with assert_cmd and update fixtures
$ENV:QPM_TEST_UPDATE="1"
& cargo test --bin qpm -- commands -- --nocapture
$ENV:QPM_TEST_UPDATE=""
