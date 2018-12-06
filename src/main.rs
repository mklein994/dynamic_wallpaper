//! Dynamic Wallpaper

/// Main binary point of entry
fn main() {
    if let Err(e) = dynamic_wallpaper::run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
