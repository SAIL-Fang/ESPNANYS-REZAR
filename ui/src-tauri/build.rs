// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // The green one-file build embeds the THX helper. It is our own binary,
    // never a Razer/THX file. If the helper has not been built yet, embed an
    // empty marker and keep the sibling/%LOCALAPPDATA% fallback available.
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repo = manifest.join("../..");
    let helper = repo.join("target/release/sail_thx.exe");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let embedded = out_dir.join("embedded_sail_thx.exe");

    if helper.is_file() {
        fs::copy(&helper, &embedded).expect("copy THX helper for embedding");
        println!("cargo:rerun-if-changed={}", helper.display());
    } else {
        fs::write(&embedded, []).expect("create empty embedded helper");
        println!(
            "cargo:warning=sail_thx.exe is absent; one-file build will use the sibling/local helper"
        );
    }
    println!("cargo:rustc-env=SAIL_THX_EMBEDDED={}", embedded.display());

    tauri_build::build()
}
