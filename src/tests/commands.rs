#[test]
fn trycmd() {
    trycmd::TestCases::new()
        .case("README.md")
        .case("test_cmd/*.toml")
        .case("test_cmd/*.trycmd")
        .run();
}