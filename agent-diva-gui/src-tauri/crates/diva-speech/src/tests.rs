//! DN-6A unit cases. Real OS stores that are absent/locked on a host
//! prove `credential_unavailable` with no fallback; injectable
//! `SecretStore` covers the presence/set/delete contract deterministically.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::assets::{AssetImportMeta, AssetStore, MAX_FILES};
use crate::config::{ConfigStore, SpeechProvider};
use crate::credentials::{CredentialPresence, CredentialStore, SecretStore};
use crate::{SpeechCode, SpeechError, SpeechResult};

fn wav_bytes(seed: u8) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&36u32.to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&[seed; 24]);
    b
}

fn mp3_bytes(seed: u8) -> Vec<u8> {
    let mut b = vec![0x49, 0x44, 0x33, seed];
    b.extend_from_slice(&[0; 32]);
    b
}

fn meta(name: &str, mime: &str) -> AssetImportMeta {
    AssetImportMeta {
        display_name: name.into(),
        mime_type: mime.into(),
    }
}

struct FakeStore {
    map: Mutex<HashMap<(String, String), String>>,
    unavailable: bool,
}

impl FakeStore {
    fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
            unavailable: false,
        }
    }
    fn broken() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
            unavailable: true,
        }
    }
}

impl SecretStore for FakeStore {
    fn set(&self, service: &str, user: &str, secret: &str) -> SpeechResult<()> {
        if self.unavailable {
            return Err(SpeechError::new(
                SpeechCode::CredentialUnavailable,
                "store unavailable",
            ));
        }
        self.map
            .lock()
            .unwrap()
            .insert((service.into(), user.into()), secret.into());
        Ok(())
    }
    fn get(&self, service: &str, user: &str) -> SpeechResult<Option<String>> {
        if self.unavailable {
            return Err(SpeechError::new(
                SpeechCode::CredentialUnavailable,
                "store unavailable",
            ));
        }
        Ok(self
            .map
            .lock()
            .unwrap()
            .get(&(service.into(), user.into()))
            .cloned())
    }
    fn delete(&self, service: &str, user: &str) -> SpeechResult<()> {
        if self.unavailable {
            return Err(SpeechError::new(
                SpeechCode::CredentialUnavailable,
                "store unavailable",
            ));
        }
        self.map
            .lock()
            .unwrap()
            .remove(&(service.into(), user.into()));
        Ok(())
    }
}

#[test]
fn credential_unavailable_no_fallback() {
    // Real OS path on this host (no default store on the dev VM) or the
    // injected-unavailable path — either way, presence is honest and no
    // plaintext fallback exists.
    let broken = CredentialStore::with_store(FakeStore::broken());
    let err = broken
        .set(SpeechProvider::SiliconFlow, "sk-test")
        .unwrap_err();
    assert_eq!(err.code, SpeechCode::CredentialUnavailable);
    assert_eq!(
        broken.presence(SpeechProvider::SiliconFlow),
        CredentialPresence::Unavailable
    );
    assert!(broken.delete(SpeechProvider::SiliconFlow).is_err());

    // The real OS store likewise never falls back — it either works or
    // reports credential_unavailable, never a file/env/sample write.
    let os = CredentialStore::os();
    let presence = os.presence(SpeechProvider::MiniMax);
    assert!(matches!(
        presence,
        CredentialPresence::Present | CredentialPresence::Absent | CredentialPresence::Unavailable
    ));
    if presence != CredentialPresence::Unavailable {
        // Store exists on this host: exercise the full roundtrip.
        os.set(SpeechProvider::MiniMax, "mm-test-key").unwrap();
        assert_eq!(
            os.presence(SpeechProvider::MiniMax),
            CredentialPresence::Present
        );
        os.delete(SpeechProvider::MiniMax).unwrap();
        assert_eq!(
            os.presence(SpeechProvider::MiniMax),
            CredentialPresence::Absent
        );
    }

    // Working fake covers the deterministic contract incl. idempotent delete.
    let store = CredentialStore::with_store(FakeStore::new());
    store.set(SpeechProvider::SiliconFlow, "sk-a").unwrap();
    assert_eq!(
        store.presence(SpeechProvider::SiliconFlow),
        CredentialPresence::Present
    );
    store.delete(SpeechProvider::SiliconFlow).unwrap();
    store.delete(SpeechProvider::SiliconFlow).unwrap(); // idempotent
    assert_eq!(
        store.presence(SpeechProvider::SiliconFlow),
        CredentialPresence::Absent
    );
}

#[test]
fn config_revision_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("speech.json");
    let mut store = ConfigStore::open(path.clone()).unwrap();
    let prefs = store.preferences().clone();
    let none_exists = |_: &str| false;

    let rev = store.update(0, prefs.clone(), &none_exists).unwrap();
    assert_eq!(rev, 1);

    // Stale base revision rejected — not a last-writer-wins overwrite.
    let err = store.update(0, prefs.clone(), &none_exists).unwrap_err();
    assert_eq!(err.code, SpeechCode::RevisionConflict);
    assert!(store.update(1, prefs, &none_exists).is_ok());

    // Corrupt file is an explicit error, not a silent reseed.
    std::fs::write(&path, "{not json").unwrap();
    let err = ConfigStore::open(path).unwrap_err();
    assert_eq!(err.code, SpeechCode::InvalidInput);
}

#[test]
fn config_reference_rules() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("speech.json");
    let mut store = ConfigStore::open(path).unwrap();

    // Provider block mismatch: minimax selected without a minimax block.
    let mut prefs = store.preferences().clone();
    prefs.tts.provider = SpeechProvider::MiniMax;
    let err = store.update(0, prefs.clone(), &|_| false).unwrap_err();
    assert_eq!(err.code, SpeechCode::InvalidInput);

    // Inline reference to a missing asset is an explicit error.
    prefs = store.preferences().clone();
    prefs.tts.siliconflow.as_mut().unwrap().reference =
        Some(crate::config::SpeechReference::Inline {
            asset_id: "va-0123456789abcdef".into(),
            transcript: "hello".into(),
        });
    let err = store.update(0, prefs.clone(), &|_| false).unwrap_err();
    assert_eq!(err.code, SpeechCode::AssetNotFound);

    // Same config with the asset present succeeds.
    assert!(store.update(0, prefs, &|_| true).is_ok());

    // base_url carrying credentials / query / fragment rejected.
    for bad in [
        "https://user:pw@api.siliconflow.cn",
        "https://api.siliconflow.cn?x=1",
        "https://api.siliconflow.cn#frag",
        "ftp://api.siliconflow.cn",
    ] {
        assert!(
            crate::config::validate_base_url("stt.base_url", bad).is_err(),
            "{bad}"
        );
    }
    assert!(crate::config::validate_base_url("stt.base_url", "https://api.siliconflow.cn").is_ok());
}

#[test]
fn asset_traversal_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let store = AssetStore::open(dir.path().join("assets")).unwrap();
    for bad in [
        "../etc/passwd",
        "..\\win.ini",
        "va-XYZ",
        "va-",
        "plain",
        "va-0123456789abcdef/extra",
    ] {
        assert!(store.read(bad).is_err(), "{bad}");
        assert!(store.delete(bad).is_err(), "{bad}");
    }
    // Well-formed but unknown id → asset_not_found, never a path probe.
    let err = store.read("va-0123456789abcdef").unwrap_err();
    assert_eq!(err.code, SpeechCode::AssetNotFound);

    // A symlink planted inside the asset dir is not a readable asset.
    let store2 = AssetStore::open(dir.path().join("assets2")).unwrap();
    let desc = store2
        .import(&wav_bytes(1), &meta("ok", "audio/wav"))
        .unwrap();
    let planted = dir.path().join("planted.wav");
    std::fs::write(&planted, b"secret-bytes").unwrap();
    // Swap the real asset file for a symlink to outside content.
    let real = dir
        .path()
        .join("assets2")
        .join(format!("{}.wav", desc.asset_id));
    std::fs::remove_file(&real).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&planted, &real).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&planted, &real).unwrap();
    let err = store2.read(&desc.asset_id).unwrap_err();
    assert_eq!(err.code, SpeechCode::AssetNotFound);
}

#[test]
fn asset_bounds() {
    let dir = tempfile::tempdir().unwrap();
    let store = AssetStore::open(dir.path().join("assets")).unwrap();

    // Invalid signature / mime mismatch / unknown mime all rejected.
    assert!(store.import(b"not audio", &meta("x", "audio/wav")).is_err());
    assert!(store
        .import(&wav_bytes(1), &meta("x", "audio/mpeg"))
        .is_err());
    assert!(store
        .import(&wav_bytes(1), &meta("x", "audio/ogg"))
        .is_err());

    // Over-10MiB file rejected.
    let mut big = wav_bytes(0);
    big.resize(10 * 1024 * 1024 + 1, 0);
    assert!(store.import(&big, &meta("big", "audio/wav")).is_err());

    // Count cap: 20 files.
    for i in 0..MAX_FILES {
        store
            .import(
                &wav_bytes(i as u8 + 1),
                &meta(&format!("n{i}"), "audio/wav"),
            )
            .unwrap();
    }
    let err = store
        .import(&mp3_bytes(9), &meta("overflow", "audio/mpeg"))
        .unwrap_err();
    assert_eq!(err.code, SpeechCode::InvalidInput);

    // List returns descriptors, never filesystem paths.
    let listed = store.list().unwrap();
    assert_eq!(listed.len(), MAX_FILES);
    let json = serde_json::to_string(&listed).unwrap();
    assert!(!json.contains("/assets"));
    assert!(!json.contains(&dir.path().display().to_string()));

    // Re-import of identical bytes updates display name, no duplicate.
    let dup = store
        .import(&wav_bytes(1), &meta("renamed", "audio/wav"))
        .unwrap();
    assert_eq!(dup.display_name, "renamed");
    assert_eq!(store.list().unwrap().len(), MAX_FILES);
}

#[test]
fn leased_delete_pending() {
    let dir = tempfile::tempdir().unwrap();
    let store = AssetStore::open(dir.path().join("assets")).unwrap();
    let desc = store
        .import(&mp3_bytes(7), &meta("voice", "audio/mpeg"))
        .unwrap();
    let id = desc.asset_id.clone();

    // A live lease turns delete into pending — file bytes stay readable
    // for the holder while a second delete reports pending again.
    let lease = store.read(&id).unwrap();
    assert_eq!(store.delete(&id).unwrap(), "pending");
    assert_eq!(store.delete(&id).unwrap(), "pending");
    assert_eq!(lease.bytes, mp3_bytes(7));
    // Pending assets don't count for new configs or reads.
    assert!(!store.exists(&id));
    assert!(store.read(&id).is_err());

    // Releasing the last lease completes the delete — file + manifest row gone.
    drop(lease);
    let listed = store.list().unwrap();
    assert!(listed.is_empty());
    let path = dir.path().join("assets").join(format!("{id}.mp3"));
    assert!(!path.exists());
    assert!(store.delete(&id).is_err());

    // Second store reopens clean (manifest reconciled, no orphans).
    let store2 = AssetStore::open(dir.path().join("assets")).unwrap();
    assert!(store2.list().unwrap().is_empty());
}

#[test]
fn manifest_and_config_atomic_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("speech.json");
    let mut cfg = ConfigStore::open(cfg_path.clone()).unwrap();
    let assets = AssetStore::open(dir.path().join("assets")).unwrap();
    let desc = assets
        .import(&wav_bytes(3), &meta("ref", "audio/wav"))
        .unwrap();

    // Persist a config referencing the real asset; reopen proves the bytes.
    let mut prefs = cfg.preferences().clone();
    prefs.tts.siliconflow.as_mut().unwrap().reference =
        Some(crate::config::SpeechReference::Inline {
            asset_id: desc.asset_id.clone(),
            transcript: "say this".into(),
        });
    let exists = |id: &str| assets.exists(id);
    cfg.update(0, prefs, &exists).unwrap();
    let reopened = ConfigStore::open(cfg_path).unwrap();
    assert_eq!(reopened.revision(), 1);
    match reopened
        .preferences()
        .tts
        .siliconflow
        .as_ref()
        .unwrap()
        .reference
        .as_ref()
        .unwrap()
    {
        crate::config::SpeechReference::Inline { asset_id, .. } => {
            assert_eq!(asset_id, &desc.asset_id)
        }
        _ => panic!("reference lost across reopen"),
    }
}
