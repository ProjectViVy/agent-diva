//! DN-6A/6B native speech state: the shell owns one `SpeechState` for the
//! process lifetime. All logic lives in the tauri-free `diva-speech`
//! crate; this module only resolves the on-disk location, wires the
//! `speech:diagnostic` sink and the commands.

pub mod commands;

use std::path::PathBuf;
use std::sync::Arc;

use diva_speech::service::{Diagnostic, SpeechService};
use diva_speech::{SpeechCode, SpeechError, SpeechResult};

/// Process speech state: the service owns the config CAS store, the OS
/// credential handle, the bounded voice-asset store and the DN-6B
/// admission/cancel registry.
pub struct SpeechState {
    pub service: SpeechService,
}

impl SpeechState {
    pub fn open(
        dir: PathBuf,
        diagnostic: Arc<dyn Fn(Diagnostic) + Send + Sync>,
    ) -> SpeechResult<Self> {
        std::fs::create_dir_all(&dir).map_err(|e| {
            SpeechError::new(
                SpeechCode::InvalidInput,
                format!("create speech dir {}: {e}", dir.display()),
            )
        })?;
        Ok(Self {
            service: SpeechService::open(dir, diagnostic)?,
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
