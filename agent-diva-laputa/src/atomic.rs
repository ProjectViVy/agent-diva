use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use crate::{LaputaError, Result};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write bytes through a same-directory temporary file and atomic rename.
pub fn atomic_write(path: impl AsRef<Path>, bytes: &[u8]) -> Result<()> {
    let path = path.as_ref();
    let parent = path
        .parent()
        .ok_or_else(|| LaputaError::io(path, invalid_input("target path has no parent")))?;

    fs::create_dir_all(parent).map_err(|source| LaputaError::io(parent, source))?;

    let temp_path = temp_path_for(path);
    let write_result = write_temp_file(&temp_path, bytes)
        .and_then(|_| fs::rename(&temp_path, path).map_err(|source| LaputaError::io(path, source)));

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    write_result
}

/// Serialize a value as pretty JSON and persist it with [`atomic_write`].
pub fn atomic_write_json<T>(path: impl AsRef<Path>, value: &T) -> Result<()>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec_pretty(value)?;
    atomic_write(path, &bytes)
}

fn write_temp_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|source| LaputaError::io(path, source))?;
    file.write_all(bytes)
        .map_err(|source| LaputaError::io(path, source))?;
    file.sync_all()
        .map_err(|source| LaputaError::io(path, source))?;
    Ok(())
}

fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("laputa-write");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);

    path.with_file_name(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        nanos + u128::from(counter)
    ))
}

fn invalid_input(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}
