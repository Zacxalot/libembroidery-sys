extern crate bindgen;
extern crate cmake;

use std::path::Path;

fn main() {
    let dst = cmake::Config::new("libembroidery").build();

    println!(
        "cargo:rustc-link-search=native={}",
        Path::new(&dst).join("lib").display()
    );

    println!("cargo:rustc-link-lib=static=embroidery");

    bindgen::Builder::default()
        .header("include/wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Failed to generate bindings")
        .write_to_file("src/ffi.rs")
        .expect("Failed to write bindings");
}
