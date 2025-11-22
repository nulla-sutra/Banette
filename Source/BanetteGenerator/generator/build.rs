use std::{env, path::PathBuf};

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_file = PathBuf::from(&crate_dir).join("bindings.h");

    cbindgen::generate(&crate_dir)
        .expect("Unable to generate bindings")
        .write_to_file(out_file);

    println!("cargo:rerun-if-changed=src/");
}
