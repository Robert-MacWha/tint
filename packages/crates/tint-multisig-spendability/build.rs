//! Builds the Go/gnark FFI library (./go/ffi) into a static archive, links
//! it into this crate, and generates Rust bindings for its exported
//! functions from the C header cgo produces (rather than hand-declaring
//! `unsafe extern "C" { ... }` blocks that could drift from the real
//! signatures). Requires `go` on PATH and libclang available (both provided
//! by `nix develop`, same as the rest of this repo's toolchain).

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let go_dir = manifest_dir.join("go");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let archive = out_dir.join("libtint_multisig.a");
    let header = out_dir.join("libtint_multisig.h");

    let status = Command::new("go")
        .current_dir(&go_dir)
        .args(["build", "-buildmode=c-archive", "-o"])
        .arg(&archive)
        .arg("./ffi")
        .status()
        .expect("failed to invoke `go build` — is Go on PATH? (`nix develop` provides it)");
    if !status.success() {
        panic!("`go build -buildmode=c-archive ./ffi` failed");
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=tint_multisig");

    // cgo-built archives pull in pthread/dl on Linux.
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=pthread");
        println!("cargo:rustc-link-lib=dylib=dl");
    }

    let bindings = bindgen::Builder::default()
        .header(header.to_str().unwrap())
        .clang_arg(format!("-I{}", go_dir.join("ffi").display()))
        .generate()
        .expect("failed to generate bindings from libtint_multisig.h");
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write bindings.rs");

    println!("cargo:rerun-if-changed=go");
}
