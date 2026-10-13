use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn main() {
    let mut files: Vec<_> = fs::read_dir("src")
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut hash = Sha256::new();
    for file in files {
        println!("cargo:rerun-if-changed={}", file.display());
        hash.update(fs::read(file).unwrap());
    }
    let mut decoder_files: Vec<_> = fs::read_dir("../orient-decoder/src")
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    decoder_files.sort();
    for file in decoder_files {
        println!("cargo:rerun-if-changed={}", file.display());
        hash.update(fs::read(file).unwrap());
    }
    for file in [
        "Cargo.toml",
        "../orient-decoder/Cargo.toml",
        "../../Cargo.lock",
    ] {
        println!("cargo:rerun-if-changed={file}");
        hash.update(fs::read(file).unwrap());
    }
    let templates = Path::new("../../src/orient/templates");
    println!("cargo:rerun-if-changed={}", templates.display());
    println!("cargo:rustc-env=ORIENT_DECODER_HASH={:x}", hash.finalize());
}
