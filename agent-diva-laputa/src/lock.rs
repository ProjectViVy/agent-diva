use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime},
};

use crate::{LaputaError, Result};

/// Lock acquisition behavior.
#[derive(Debug, Clone, Copy)]
pub struct LockOptions {
    pub timeout: Duration,
    pub stale_after: Duration,
    pub retry_interval: Duration,
}

impl Default for LockOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            stale_after: Duration::from_secs(60 * 5),
            retry_interval: Duration::from_millis(10),
        }
    }
}

/// Cross-platform lock-file guard removed on drop.
#[derive(Debug)]
pub struct LaputaLock {
    path: PathBuf,
}

impl LaputaLock {
    /// Acquire a lock with timeout and stale lock recovery.
    pub fn acquire(path: impl AsRef<Path>, options: LockOptions) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| LaputaError::io(parent, source))?;
        }

        let deadline = Instant::now() + options.timeout;
        loop {
            match try_create_lock(path) {
                Ok(()) => {
                    return Ok(Self {
                        path: path.to_path_buf(),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    recover_stale_lock(path, options.stale_after)?;
                    if Instant::now() >= deadline {
                        return Err(LaputaError::LockTimeout {
                            path: path.to_path_buf(),
                        });
                    }
                    thread::sleep(options.retry_interval);
                }
                Err(source) => return Err(LaputaError::io(path, source)),
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for LaputaLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn try_create_lock(path: &Path) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    writeln!(
        file,
        "pid={}\ncreated_at={:?}",
        std::process::id(),
        SystemTime::now()
    )?;
    file.sync_all()?;
    Ok(())
}

fn recover_stale_lock(path: &Path, stale_after: Duration) -> Result<()> {
    let metadata = fs::metadata(path).map_err(|source| LaputaError::io(path, source))?;
    let modified = metadata
        .modified()
        .map_err(|source| LaputaError::io(path, source))?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);

    if age >= stale_after {
        fs::remove_file(path).map_err(|source| LaputaError::io(path, source))?;
    }

    Ok(())
}
