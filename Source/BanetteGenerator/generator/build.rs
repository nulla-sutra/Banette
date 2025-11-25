/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_file = PathBuf::from(&crate_dir).join("bindings.h");
    //
    cbindgen::generate(&crate_dir)
        .expect("Unable to generate bindings")
        .write_to_file(&out_file);

    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}
