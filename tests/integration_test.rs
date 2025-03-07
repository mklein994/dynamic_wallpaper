#[test]
fn run_smoke_test() {
    // TODO: Audit that the environment access only happens in single-threaded code.
    unsafe {
        std::env::set_var(
            "DYNAMIC_WALLPAPER_CONFIG",
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_config.toml"),
        );
    };
    let index = dynamic_wallpaper::run().unwrap();

    assert_eq!(3, index);
}
