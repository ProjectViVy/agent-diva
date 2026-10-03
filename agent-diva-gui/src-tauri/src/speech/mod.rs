//! DN-6A native speech state: the shell owns one `SpeechState` for the
//! process lifetime. All logic lives in the tauri-free `diva-speech`
//! crate; this module only resolves the on-disk location and wires the
//! commands.

pub mod commands;

use std::path::PathBuf;
use std::sync::Mutex;

use diva_speech::assets::AssetStore;
use diva_speech::config::ConfigStore;
use diva_speech::credentials::CredentialStore;
use diva_speech::{SpeechCode, SpeechError, SpeechResult};

/// Process speech state: config CAS store, OS credential handle, and the
/// bounded voice-asset store. `window_context` is reserved for DN-6B's
/// `speech_context_set`; config reads report it as `null` until then.
pub struct SpeechState {
    pub config: Mutex<ConfigStore>,
    pub credentials: CredentialStore,
    pub assets: AssetStore,
    pub window_context: Mutex<Option<serde_json::Value>>,
}

impl SpeechState {
    pub fn open(dir: PathBuf) -> SpeechResult<Self> {
        std::fs::create_dir_all(&dir).map_err(|e| {
            SpeechError::new(
                SpeechCode::InvalidInput,
                format!("create speech dir {}: {e}", dir.display()),
            )
        })?;
        Ok(Self {
            config: Mutex::new(ConfigStore::open(dir.join("speech.json"))?),
            credentials: CredentialStore::os(),
            assets: AssetStore::open(dir.join("assets"))?,
            window_context: Mutex::new(None),
        })
    }
}

/// `DIVA_SPEECH_DIR` (dev/test) or `<app_config_dir>/speech`.
pub fn speech_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> SpeechResult<PathBuf> {
    if let Ok(dir) = std::env::var("DIVA_SPEECH_DIR") {
        return Ok(PathBuf::from(dir));
    }
    app.path()
        .app_config_dir()
        .map(|d| d.join("speech"))
        .map_err(|e| SpeechError::new(SpeechCode::InvalidInput, format!("config dir: {e}")))
}
