use crate::{resolve, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::os::unix::fs::MetadataExt;
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub fn root() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache")
        })
        .join("orient")
}
pub fn read(path: &Path) -> Result<Value> {
    Ok(serde_json::from_reader(File::open(path)?)?)
}
pub fn write(path: &Path, value: &Value) -> Result<()> {
    let folder = path.parent().ok_or("Invalid cache path")?;
    fs::create_dir_all(folder)?;
    let mut file = tempfile::NamedTempFile::new_in(folder)?;
    serde_json::to_writer(&mut file, value)?;
    file.flush()?;
    file.persist(path)?;
    Ok(())
}
pub fn digest(path: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    let mut file = File::open(path)?;
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let len = file.read(&mut buffer)?;
        if len == 0 {
            break;
        }
        hash.update(&buffer[..len]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn identity(path: &Path, settings: &Value) -> Result<String> {
    let meta = path.metadata()?;
    let identity = json!([
        resolve(path)?,
        meta.dev(),
        meta.ino(),
        meta.size(),
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec(),
        settings,
        crate::engine_id(),
        env!("ORIENT_DECODER_HASH")
    ]);
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&identity)?)
    ))
}
