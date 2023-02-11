//! Dynamic Wallpaper

/// Main binary point of entry
fn main() {
    match dynamic_wallpaper::run() {
        Ok(index) => println!("{index}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
