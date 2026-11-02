//! Desktop-owned session identity and *unconfirmed* sends, never task history.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendRecord {
    pub request_id: String,
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopState {
    pub schema: u32,
    pub session: String,
    pub outbox: BTreeMap<String, SendRecord>,
}

pub struct Store {
    pub path: PathBuf,
    pub value: DesktopState,
}
impl Store {
    pub fn open(directory: &Path) -> io::Result<Self> {
        fs::create_dir_all(directory)?;
        let path = directory.join("state.json");
        let value = match fs::read(&path) {
            Ok(bytes) => {
                let state: DesktopState =
                    serde_json::from_slice(&bytes).map_err(io::Error::other)?;
                if state.schema != 1
                    || !valid_session(&state.session)
                    || state.outbox.len() > 128
                    || state
                        .outbox
                        .iter()
                        .any(|(k, v)| k != &v.request_id || !valid_record(v))
                {
                    return Err(io::Error::other(
                        "unsupported or corrupt desktop state; retained original file",
                    ));
                }
                state
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let state = DesktopState {
                    schema: 1,
                    session: format!(
                        "{}-{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        std::process::id()
                    ),
                    outbox: BTreeMap::new(),
                };
                atomic_write(&path, &serde_json::to_vec(&state)?)?;
                state
            }
            Err(e) => return Err(e),
        };
        Ok(Self { path, value })
    }
    pub fn admit(&mut self, record: SendRecord) -> io::Result<()> {
        if !valid_record(&record) {
            return Err(io::Error::other("invalid or oversized send record"));
        }
        if let Some(existing) = self.value.outbox.get(&record.request_id) {
            return if existing == &record {
                Ok(())
            } else {
                Err(io::Error::other(
                    "request ID has different unconfirmed content",
                ))
            };
        }
        if self.value.outbox.len() >= 128 {
            return Err(io::Error::other(
                "outbox is full; reconnect to reconcile sends",
            ));
        }
        let mut next = self.value.clone();
        next.outbox.insert(record.request_id.clone(), record);
        self.commit(next)
    }
    pub fn acknowledge(&mut self, key: &str) -> io::Result<()> {
        if !self.value.outbox.contains_key(key) {
            return Ok(());
        }
        let mut next = self.value.clone();
        next.outbox.remove(key);
        self.commit(next)
    }
    fn commit(&mut self, next: DesktopState) -> io::Result<()> {
        atomic_write(&self.path, &serde_json::to_vec(&next)?)?;
        self.value = next;
        Ok(())
    }
}
fn valid_session(value: &str) -> bool {
    value.split_once('-').is_some_and(|(a, b)| {
        !a.is_empty()
            && !b.is_empty()
            && a.bytes().all(|c| c.is_ascii_digit())
            && b.bytes().all(|c| c.is_ascii_digit())
    })
}
fn valid_record(record: &SendRecord) -> bool {
    !record.request_id.is_empty()
        && record.request_id.len() <= 256
        && !record.text.trim().is_empty()
        && record.text.len() <= 512 * 1024
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{
                MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
            };
            let from: Vec<_> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe {
                MoveFileExW(
                    from.as_ptr(),
                    to.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        #[cfg(not(windows))]
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_before_reopen_and_does_not_mutate_after_write_failure() {
        let root = tempfile::tempdir().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let session = store.value.session.clone();
        let record = SendRecord {
            request_id: "stable".into(),
            text: "hello".into(),
        };
        store.admit(record.clone()).unwrap();
        store.admit(record).unwrap();
        assert_eq!(Store::open(root.path()).unwrap().value.outbox.len(), 1);
        store.path = root.path().join("absent/state.json");
        assert!(store.acknowledge("stable").is_err());
        assert_eq!(store.value.outbox.len(), 1);
        let restored = Store::open(root.path()).unwrap();
        assert_eq!(restored.value.session, session);
        assert_eq!(restored.value.outbox.len(), 1);
    }
    #[test]
    fn refuses_corruption_and_unknown_schema_without_replacement() {
        let root = tempfile::tempdir().unwrap();
        for text in ["{broken", r#"{"schema":9,"session":"123-456","outbox":{}}"#] {
            fs::write(root.path().join("state.json"), text).unwrap();
            assert!(Store::open(root.path()).is_err());
            assert_eq!(
                fs::read_to_string(root.path().join("state.json")).unwrap(),
                text
            );
        }
    }
}
