//! Bounded owned reference-asset storage. Import takes raw bytes + a
//! display name/MIME (from the `x-diva-asset-meta` IPC header) — never a
//! path or URL. Native IDs are digest-derived; filenames are DERIVED from
//! the ID, so neither caller names nor manifest content can reach outside
//! the asset dir. Deletes never race readers: an in-use asset is marked
//! `delete_pending` and the last released lease completes the removal.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::atomic_write;
use crate::{SpeechCode, SpeechError, SpeechResult};

pub const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_FILES: usize = 20;
pub const MAX_TOTAL_BYTES: u64 = 100 * 1024 * 1024;
pub const MAX_DISPLAY_NAME: usize = 128;
const MANIFEST: &str = "manifest.json";

/// Caller-supplied metadata (`x-diva-asset-meta`, bounded by the shell).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetImportMeta {
    pub display_name: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetEntry {
    pub asset_id: String,
    pub display_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub digest_sha256: String,
    #[serde(default)]
    pub delete_pending: bool,
}

/// Public descriptor — no filesystem path, no secret fields.
#[derive(Debug, Clone, Serialize)]
pub struct AssetDescriptor {
    pub asset_id: String,
    pub display_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub digest_sha256: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    assets: Vec<AssetEntry>,
}

fn invalid_input(msg: impl Into<String>) -> SpeechError {
    SpeechError::new(SpeechCode::InvalidInput, msg)
}

/// Asset ids are native-generated `va-<16 hex>`; anything else — including
/// separators, `..`, or foreign namespaces — is rejected before it can
/// ever reach a path join.
pub fn validate_asset_id(asset_id: &str) -> SpeechResult<()> {
    let ok = asset_id.len() == 19
        && asset_id.starts_with("va-")
        && asset_id[3..]
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase());
    if ok {
        Ok(())
    } else {
        Err(invalid_input(format!("malformed asset_id {asset_id:?}")))
    }
}

fn file_name(asset_id: &str, mime_type: &str) -> String {
    let ext = if mime_type.contains("wav") {
        "wav"
    } else {
        "mp3"
    };
    format!("{asset_id}.{ext}")
}

fn sniff_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return Some("audio/wav");
    }
    if bytes.len() >= 3 && &bytes[0..3] == b"ID3" {
        return Some("audio/mpeg");
    }
    if bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0 {
        return Some("audio/mpeg");
    }
    None
}

struct AssetInner {
    manifest: Manifest,
    leases: HashMap<String, u32>,
}

/// Read lease: holds the bytes plus a drop hook that completes pending
/// deletes once the last reader releases.
pub struct AssetLease {
    inner: Arc<Mutex<AssetInner>>,
    dir: PathBuf,
    id: String,
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

impl Drop for AssetLease {
    fn drop(&mut self) {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let Some(count) = inner.leases.get_mut(&self.id) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            inner.leases.remove(&self.id);
            if inner
                .manifest
                .assets
                .iter()
                .any(|a| a.asset_id == self.id && a.delete_pending)
            {
                complete_delete(&self.dir, &mut inner, &self.id);
            }
        }
    }
}

fn complete_delete(dir: &Path, inner: &mut AssetInner, asset_id: &str) {
    if let Some(pos) = inner
        .manifest
        .assets
        .iter()
        .position(|a| a.asset_id == asset_id)
    {
        let entry = inner.manifest.assets.remove(pos);
        let _ = fs::remove_file(dir.join(file_name(&entry.asset_id, &entry.mime_type)));
        let bytes = serde_json::to_vec_pretty(&inner.manifest).unwrap_or_default();
        let _ = atomic_write(&dir.join(MANIFEST), &bytes);
    }
}

pub struct AssetStore {
    dir: PathBuf,
    inner: Arc<Mutex<AssetInner>>,
}

impl std::fmt::Debug for AssetStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetStore")
            .field("dir", &self.dir)
            .finish()
    }
}

impl std::fmt::Debug for AssetLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetLease")
            .field("id", &self.id)
            .field("mime_type", &self.mime_type)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

impl AssetStore {
    /// Open the store: reconcile the manifest with the directory (orphan
    /// manifest rows whose files are gone are dropped on open — a row
    /// without bytes is a lie), and delete orphan files not referenced by
    /// any entry.
    pub fn open(dir: PathBuf) -> SpeechResult<Self> {
        fs::create_dir_all(&dir)
            .map_err(|e| invalid_input(format!("create {}: {e}", dir.display())))?;
        let manifest_path = dir.join(MANIFEST);
        let mut manifest = if manifest_path.exists() {
            let text = fs::read_to_string(&manifest_path)
                .map_err(|e| invalid_input(format!("read manifest: {e}")))?;
            serde_json::from_str::<Manifest>(&text)
                .map_err(|e| invalid_input(format!("manifest corrupt: {e}")))?
        } else {
            Manifest { assets: Vec::new() }
        };

        manifest
            .assets
            .retain(|a| dir.join(file_name(&a.asset_id, &a.mime_type)).is_file());
        let keep: HashSet<String> = manifest
            .assets
            .iter()
            .map(|a| file_name(&a.asset_id, &a.mime_type))
            .collect();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name == MANIFEST || keep.contains(&name) {
                    continue;
                }
                // Orphan file — remove only regular files we own.
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }

        Ok(Self {
            dir,
            inner: Arc::new(Mutex::new(AssetInner {
                manifest,
                leases: HashMap::new(),
            })),
        })
    }

    /// Bounded import: signature-verified WAV/MP3 bytes under size,
    /// count, and total caps. Re-import of identical bytes updates the
    /// display name — one asset per digest, never a duplicate.
    pub fn import(&self, bytes: &[u8], meta: &AssetImportMeta) -> SpeechResult<AssetDescriptor> {
        let mut inner = self.lock()?;
        let sniffed = sniff_mime(bytes).ok_or_else(|| {
            SpeechError::new(SpeechCode::InvalidAudio, "bytes are not WAV or MP3")
        })?;
        let declared = normalize_mime(&meta.mime_type)
            .ok_or_else(|| invalid_input(format!("unsupported mime_type {:?}", meta.mime_type)))?;
        if declared != sniffed {
            return Err(SpeechError::new(
                SpeechCode::InvalidAudio,
                format!("mime_type {declared} does not match bytes ({sniffed})"),
            ));
        }
        if bytes.is_empty() || bytes.len() > MAX_FILE_BYTES {
            return Err(SpeechError::new(
                SpeechCode::InvalidAudio,
                format!("asset size {} outside 1..={MAX_FILE_BYTES}", bytes.len()),
            ));
        }
        if meta.display_name.trim().is_empty() || meta.display_name.len() > MAX_DISPLAY_NAME {
            return Err(invalid_input("display_name empty or overlong"));
        }

        let digest = hex_sha256(bytes);
        let asset_id = format!("va-{}", &digest[..16]);
        if let Some(pos) = inner
            .manifest
            .assets
            .iter()
            .position(|a| a.asset_id == asset_id)
        {
            inner.manifest.assets[pos].display_name = meta.display_name.clone();
            inner.manifest.assets[pos].delete_pending = false;
            let stored = inner.manifest.assets[pos].clone();
            self.persist(&mut inner)?;
            return Ok(descriptor(&stored));
        }

        if inner.manifest.assets.len() >= MAX_FILES {
            return Err(SpeechError::new(
                SpeechCode::InvalidInput,
                format!(
                    "asset count {} at cap {MAX_FILES}",
                    inner.manifest.assets.len()
                ),
            ));
        }
        let total: u64 = inner.manifest.assets.iter().map(|a| a.size_bytes).sum();
        if total + bytes.len() as u64 > MAX_TOTAL_BYTES {
            return Err(SpeechError::new(
                SpeechCode::InvalidInput,
                "asset store total cap exceeded",
            ));
        }

        let entry = AssetEntry {
            asset_id: asset_id.clone(),
            display_name: meta.display_name.clone(),
            mime_type: sniffed.to_string(),
            size_bytes: bytes.len() as u64,
            digest_sha256: digest,
            delete_pending: false,
        };
        atomic_write(&self.dir.join(file_name(&asset_id, sniffed)), bytes)?;
        inner.manifest.assets.push(entry);
        self.persist(&mut inner)?;
        let stored = inner.manifest.assets.last().expect("just pushed");
        Ok(descriptor(stored))
    }

    pub fn list(&self) -> SpeechResult<Vec<AssetDescriptor>> {
        let inner = self.lock()?;
        Ok(inner.manifest.assets.iter().map(descriptor).collect())
    }

    /// Known-asset check for config validation — pending deletes do not
    /// count (a new config must not pin a dying asset).
    pub fn exists(&self, asset_id: &str) -> bool {
        validate_asset_id(asset_id).is_ok()
            && self
                .inner
                .lock()
                .map(|inner| {
                    inner
                        .manifest
                        .assets
                        .iter()
                        .any(|a| a.asset_id == asset_id && !a.delete_pending)
                })
                .unwrap_or(false)
    }

    /// Read bytes for explicit local preview — takes a lease so a delete
    /// issued mid-read completes only on release.
    pub fn read(&self, asset_id: &str) -> SpeechResult<AssetLease> {
        validate_asset_id(asset_id)?;
        let mut inner = self.lock()?;
        let entry = inner
            .manifest
            .assets
            .iter()
            .find(|a| a.asset_id == asset_id && !a.delete_pending)
            .cloned()
            .ok_or_else(|| SpeechError::new(SpeechCode::AssetNotFound, "unknown asset_id"))?;
        let path = self.dir.join(file_name(&entry.asset_id, &entry.mime_type));
        let file_type = fs::symlink_metadata(&path)
            .map_err(|_| SpeechError::new(SpeechCode::AssetNotFound, "asset file missing"))?
            .file_type();
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(SpeechError::new(
                SpeechCode::AssetNotFound,
                "asset file is not a regular file",
            ));
        }
        let bytes = fs::read(&path).map_err(|e| invalid_input(format!("read asset: {e}")))?;
        *inner.leases.entry(asset_id.to_string()).or_insert(0) += 1;
        Ok(AssetLease {
            inner: self.inner.clone(),
            dir: self.dir.clone(),
            id: asset_id.to_string(),
            bytes,
            mime_type: entry.mime_type,
        })
    }

    /// Delete: immediate when unleased; `pending` while a reader holds a
    /// lease — the last release completes the removal.
    pub fn delete(&self, asset_id: &str) -> SpeechResult<&'static str> {
        validate_asset_id(asset_id)?;
        let mut inner = self.lock()?;
        let Some(pos) = inner
            .manifest
            .assets
            .iter()
            .position(|a| a.asset_id == asset_id)
        else {
            return Err(SpeechError::new(
                SpeechCode::AssetNotFound,
                "unknown asset_id",
            ));
        };
        if inner.leases.get(asset_id).copied().unwrap_or(0) > 0 {
            inner.manifest.assets[pos].delete_pending = true;
            self.persist(&mut inner)?;
            return Ok("pending");
        }
        let entry = inner.manifest.assets.remove(pos);
        fs::remove_file(self.dir.join(file_name(&entry.asset_id, &entry.mime_type)))
            .map_err(|e| invalid_input(format!("remove asset: {e}")))?;
        self.persist(&mut inner)?;
        Ok("deleted")
    }

    fn lock(&self) -> SpeechResult<std::sync::MutexGuard<'_, AssetInner>> {
        self.inner
            .lock()
            .map_err(|_| SpeechError::new(SpeechCode::Busy, "asset store lock poisoned"))
    }

    fn persist(&self, inner: &mut AssetInner) -> SpeechResult<()> {
        let bytes = serde_json::to_vec_pretty(&inner.manifest)
            .map_err(|e| invalid_input(format!("encode manifest: {e}")))?;
        atomic_write(&self.dir.join(MANIFEST), &bytes)
    }
}

fn descriptor(entry: &AssetEntry) -> AssetDescriptor {
    AssetDescriptor {
        asset_id: entry.asset_id.clone(),
        display_name: entry.display_name.clone(),
        mime_type: entry.mime_type.clone(),
        size_bytes: entry.size_bytes,
        digest_sha256: entry.digest_sha256.clone(),
        status: if entry.delete_pending {
            "pending"
        } else {
            "active"
        },
    }
}

fn normalize_mime(mime: &str) -> Option<&'static str> {
    match mime.trim().to_ascii_lowercase().as_str() {
        "audio/wav" | "audio/x-wav" | "audio/wave" | "audio/vnd.wave" => Some("audio/wav"),
        "audio/mpeg" | "audio/mp3" | "audio/mpeg3" => Some("audio/mpeg"),
        _ => None,
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}
