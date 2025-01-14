use std::env;

use winresource::WindowsResource;

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}\\libs", dir);

    let _ = WindowsResource::new().set_icon("res/dorg.ico").compile();
}