#[test]
#[cfg(feature = "cli")]
fn trycmd() {
    // #[cfg(test)]
    // let bin = trycmd::cargo::cargo_bin!("qpm_cli");
    // #[cfg(not(test))]
    let bin = trycmd::cargo::cargo_bin("qpm");


    assert!(bin.exists(), "Binary not found: {:?}", bin);

    trycmd::TestCases::new()
        .default_bin_path(bin)
        .case("README.md")
        .pass("test_cmd/*.toml")
        .pass("test_cmd/*.trycmd")
        .run();
}
