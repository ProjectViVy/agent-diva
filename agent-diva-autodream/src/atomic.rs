use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use crate::{AutoDreamError, Result};

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AutoDreamError::InvalidState("path has no parent".to_string()))?;
    fs::create_dir_all(parent).map_err(|source| AutoDreamError::io(parent, source))?;
    let temp = temp_path(path);
    {
        let mut file =
            fs::File::create(&temp).map_err(|source| AutoDreamError::io(&temp, source))?;
        file.write_all(bytes)
            .map_err(|source| AutoDreamError::io(&temp, source))?;
        file.sync_all()
            .map_err(|source| AutoDreamError::io(&temp, source))?;
    }
    fs::rename(&temp, path).map_err(|source| AutoDreamError::io(path, source))?;
    Ok(())
}

pub(crate) fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    atomic_write(path, &bytes)
}

fn temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("autodream");
    path.with_file_name(format!(".{file_name}.tmp"))
}
