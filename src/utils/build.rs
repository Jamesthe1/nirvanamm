use std::env;

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo::rustc-link-search={}/libs/libxdelta3.dll", dir);
}