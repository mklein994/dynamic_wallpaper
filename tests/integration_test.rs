#[test]
fn run_smoke_test() {
    std::env::set_var(
        "DYNAMIC_WALLPAPER_CONFIG",
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_config.toml"),
    );
    let index = dynamic_wallpaper::run().unwrap();

    assert_eq!(5, index);
}
