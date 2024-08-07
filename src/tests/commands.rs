#[test]
fn trycmd() {
    trycmd::TestCases::new()
        .case("README.md")
        .pass("test_cmd/*.toml")
        .pass("test_cmd/*.trycmd")
        .run();
}
