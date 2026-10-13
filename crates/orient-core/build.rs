use sha2::{Digest, Sha256};
use std::{env, fs, path::Path};
fn hash_sources(path: &Path, hash: &mut Sha256) {
    let mut files: Vec<_> = fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    for file in files {
        if file.is_dir() {
            hash_sources(&file, hash)
        } else if file.extension().is_some_and(|e| e == "rs") {
            println!("cargo:rerun-if-changed={}", file.display());
            hash.update(fs::read(&file).unwrap());
        }
    }
}
fn main() {
    let mut hash = Sha256::new();
    hash_sources(Path::new("src"), &mut hash);
    let decoder = Path::new("../../src/orient/extract.py");
    println!("cargo:rerun-if-changed={}", decoder.display());
    hash.update(fs::read(decoder).unwrap());
    let version = env::var("CARGO_PKG_VERSION")
        .unwrap()
        .replace("-dev.", ".dev");
    let id = format!("orient-{version}-{:x}", hash.finalize());
    println!("cargo:rustc-env=ORIENT_ENGINE_ID={}", &id[..id.len() - 52]);
    println!("cargo:rerun-if-changed=build.rs");
}
