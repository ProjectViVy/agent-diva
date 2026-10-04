//! DN-6A command surface: preferences CAS, credential set/delete, and
//! bounded voice assets. DN-6B adds the request commands
//! (transcribe/synthesize/cancel/context_set); DN-6C adds the settings UI.

use std::sync::Arc;

use diva_speech::assets::{AssetDescriptor, AssetImportMeta};
use diva_speech::config::{SpeechPreferences, SPEECH_SCHEMA};
use diva_speech::credentials::CredentialPresence;
use diva_speech::registry::SpeechContext;
use diva_speech::service::{SpeechIdentity, TranscribeReply};
use diva_speech::{SpeechCode, SpeechError, SpeechResult};
use serde::{Deserialize, Serialize};
use tauri::ipc::{InvokeBody, InvokeError, Request, Response};
use tauri::{State, WebviewWindow};

use super::SpeechState;

/// Metadata header cap for `voice_asset_import` (`x-diva-asset-meta`).
const ASSET_META_MAX_BYTES: usize = 2048;

/// C2-4 main-window-only gate: speech state belongs to the primary
/// window; every other caller is rejected before touching state.
fn require_main_window(window: &WebviewWindow) -> SpeechResult<()> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(SpeechError::new(
            SpeechCode::InvalidInput,
            "speech commands are only callable from the main window",
        ))
    }
}

fn presence_str(p: CredentialPresence) -> &'static str {
    match p {
        CredentialPresence::Present => "present",
        CredentialPresence::Absent => "absent",
        CredentialPresence::Unavailable => "unavailable",
    }
}

/// Credential presence readback: never the secret itself.
#[derive(Debug, Serialize)]
pub struct CredentialStateReadback {
    pub siliconflow: &'static str,
    pub minimax: &'static str,
}

/// `speech_config_get` response shape — secrets never appear.
#[derive(Debug, Serialize)]
pub struct SpeechConfigReadback {
    pub schema: &'static str,
    pub revision: u64,
    pub preferences: SpeechPreferences,
    pub credential_state: CredentialStateReadback,
    pub window_context: Option<SpeechContext>,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn speech_config_get(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
) -> Result<SpeechConfigReadback, InvokeError> {
    (|| -> SpeechResult<SpeechConfigReadback> {
        require_main_window(&window)?;
        let config = state
            .service
            .config()
            .lock()
            .map_err(|_| SpeechError::new(SpeechCode::InvalidInput, "config lock poisoned"))?;
        Ok(SpeechConfigReadback {
            schema: SPEECH_SCHEMA,
            revision: config.revision(),
            preferences: config.preferences().clone(),
            credential_state: CredentialStateReadback {
                siliconflow: presence_str(
                    state
                        .service
                        .credentials()
                        .presence(diva_speech::config::SpeechProvider::SiliconFlow),
                ),
                minimax: presence_str(
                    state
                        .service
                        .credentials()
                        .presence(diva_speech::config::SpeechProvider::MiniMax),
                ),
            },
            window_context: state.service.context(),
        })
    })()
    .map_err(InvokeError::from)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn speech_config_update(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
    base_revision: u64,
    preferences: SpeechPreferences,
) -> Result<SpeechConfigReadback, InvokeError> {
    (|| -> SpeechResult<SpeechConfigReadback> {
        require_main_window(&window)?;
        let mut config = state
            .service
            .config()
            .lock()
            .map_err(|_| SpeechError::new(SpeechCode::InvalidInput, "config lock poisoned"))?;
        let asset_exists = |id: &str| state.service.assets().exists(id);
        config.update(base_revision, preferences, &asset_exists)?;
        Ok(SpeechConfigReadback {
            schema: SPEECH_SCHEMA,
            revision: config.revision(),
            preferences: config.preferences().clone(),
            credential_state: CredentialStateReadback {
                siliconflow: presence_str(
                    state
                        .service
                        .credentials()
                        .presence(diva_speech::config::SpeechProvider::SiliconFlow),
                ),
                minimax: presence_str(
                    state
                        .service
                        .credentials()
                        .presence(diva_speech::config::SpeechProvider::MiniMax),
                ),
            },
            window_context: state.service.context(),
        })
    })()
    .map_err(InvokeError::from)
}

/// `speech_credential_set` / `speech_credential_delete` result: presence
/// only — the key itself never re-enters the response.
#[derive(Debug, Serialize)]
pub struct CredentialMutationResult {
    pub provider: &'static str,
    pub present: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn speech_credential_set(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
    provider: String,
    key: String,
) -> Result<CredentialMutationResult, InvokeError> {
    (|| -> SpeechResult<CredentialMutationResult> {
        require_main_window(&window)?;
        let p = diva_speech::config::SpeechProvider::parse(&provider)?;
        if key.is_empty() || key.len() > 4096 {
            return Err(SpeechError::new(
                SpeechCode::InvalidInput,
                "key must be 1..=4096 bytes",
            ));
        }
        state.service.credentials().set(p, &key)?;
        Ok(CredentialMutationResult {
            provider: p.as_str(),
            present: true,
        })
    })()
    .map_err(InvokeError::from)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn speech_credential_delete(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
    provider: String,
) -> Result<CredentialMutationResult, InvokeError> {
    (|| -> SpeechResult<CredentialMutationResult> {
        require_main_window(&window)?;
        let p = diva_speech::config::SpeechProvider::parse(&provider)?;
        state.service.credentials().delete(p)?;
        Ok(CredentialMutationResult {
            provider: p.as_str(),
            present: false,
        })
    })()
    .map_err(InvokeError::from)
}

/// `voice_asset_import`: raw WAV/MP3 bytes in the request body,
/// `x-diva-asset-meta` (`{display_name, mime_type}`, ≤ 2 KiB) in headers.
/// Returns the descriptor; audio bytes never appear in JSON.
#[tauri::command(rename_all = "snake_case")]
pub async fn voice_asset_import(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
    request: Request<'_>,
) -> Result<AssetDescriptor, InvokeError> {
    (|| -> SpeechResult<AssetDescriptor> {
        require_main_window(&window)?;
        let meta_header = request.headers().get("x-diva-asset-meta").ok_or_else(|| {
            SpeechError::new(SpeechCode::InvalidInput, "missing x-diva-asset-meta header")
        })?;
        let meta_text = meta_header.to_str().map_err(|_| {
            SpeechError::new(SpeechCode::InvalidInput, "x-diva-asset-meta is not UTF-8")
        })?;
        if meta_text.len() > ASSET_META_MAX_BYTES {
            return Err(SpeechError::new(
                SpeechCode::InvalidInput,
                "x-diva-asset-meta exceeds 2 KiB",
            ));
        }
        let meta: AssetImportMeta = serde_json::from_str(meta_text).map_err(|e| {
            SpeechError::new(
                SpeechCode::InvalidInput,
                format!("bad x-diva-asset-meta: {e}"),
            )
        })?;
        let bytes = match request.body() {
            InvokeBody::Raw(b) => b.as_slice(),
            InvokeBody::Json(_) => {
                return Err(SpeechError::new(
                    SpeechCode::InvalidInput,
                    "voice_asset_import requires a raw request body",
                ))
            }
        };
        state.service.assets().import(bytes, &meta)
    })()
    .map_err(InvokeError::from)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn voice_asset_list(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
) -> Result<Vec<AssetDescriptor>, InvokeError> {
    (|| -> SpeechResult<Vec<AssetDescriptor>> {
        require_main_window(&window)?;
        state.service.assets().list()
    })()
    .map_err(InvokeError::from)
}

/// `voice_asset_read`: raw bytes via `ipc::Response` — never JSON-encoded.
/// The lease is dropped once the body is handed to IPC; pending deletes
/// complete only after the bytes leave the store.
#[tauri::command(rename_all = "snake_case")]
pub async fn voice_asset_read(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
    asset_id: String,
) -> Result<Response, InvokeError> {
    (|| -> SpeechResult<Response> {
        require_main_window(&window)?;
        let lease = state.service.assets().read(&asset_id)?;
        Ok(Response::new(lease.bytes.clone()))
    })()
    .map_err(InvokeError::from)
}

/// `voice_asset_delete`: returns "deleted" or "pending" (a live lease
/// holds the asset; the delete completes on lease release).
#[tauri::command(rename_all = "snake_case")]
pub async fn voice_asset_delete(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
    asset_id: String,
) -> Result<String, InvokeError> {
    (|| -> SpeechResult<String> {
        require_main_window(&window)?;
        state.service.assets().delete(&asset_id).map(String::from)
    })()
    .map_err(InvokeError::from)
}

// ---------- DN-6B request lane ----------

/// `speech_context_set` readback — the echoed trusted tuple.
#[derive(Debug, Serialize)]
pub struct SpeechContextReadback {
    pub session_id: String,
    pub generation: u64,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn speech_context_set(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
    session_id: String,
    generation: u64,
) -> Result<SpeechContextReadback, InvokeError> {
    (|| -> SpeechResult<SpeechContextReadback> {
        require_main_window(&window)?;
        let ctx = state.service.set_context(&session_id, generation)?;
        Ok(SpeechContextReadback {
            session_id: ctx.session_id,
            generation: ctx.generation,
        })
    })()
    .map_err(InvokeError::from)
}

/// `x-diva-speech-meta` header for `speech_transcribe`:
/// `{identity, mime_type}` — bounded at 2 KiB, IPC metadata only, never
/// provider Authorization.
const SPEECH_META_MAX_BYTES: usize = 2048;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TranscribeMeta {
    identity: SpeechIdentity,
    mime_type: String,
}

/// `speech_transcribe`: Raw WAV body + metadata header → JSON reply.
#[tauri::command(rename_all = "snake_case")]
pub async fn speech_transcribe(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
    request: Request<'_>,
) -> Result<TranscribeReply, InvokeError> {
    require_main_window(&window).map_err(InvokeError::from)?;
    let meta_header = request.headers().get("x-diva-speech-meta").ok_or_else(|| {
        InvokeError::from(SpeechError::new(
            SpeechCode::InvalidInput,
            "missing x-diva-speech-meta header",
        ))
    })?;
    let meta_text = meta_header.to_str().map_err(|_| {
        InvokeError::from(SpeechError::new(
            SpeechCode::InvalidInput,
            "x-diva-speech-meta is not UTF-8",
        ))
    })?;
    if meta_text.len() > SPEECH_META_MAX_BYTES {
        return Err(InvokeError::from(SpeechError::new(
            SpeechCode::InvalidInput,
            "x-diva-speech-meta exceeds 2 KiB",
        )));
    }
    let meta: TranscribeMeta = serde_json::from_str(meta_text).map_err(|e| {
        InvokeError::from(SpeechError::new(
            SpeechCode::InvalidInput,
            format!("bad x-diva-speech-meta: {e}"),
        ))
    })?;
    if meta.mime_type != "audio/wav" {
        return Err(InvokeError::from(SpeechError::new(
            SpeechCode::InvalidAudio,
            "x-diva-speech-meta mime_type must be audio/wav",
        )));
    }
    let bytes = match request.body() {
        InvokeBody::Raw(b) => b.clone(),
        InvokeBody::Json(_) => {
            return Err(InvokeError::from(SpeechError::new(
                SpeechCode::InvalidInput,
                "speech_transcribe requires a raw request body",
            )))
        }
    };
    state
        .service
        .transcribe(meta.identity, bytes)
        .await
        .map_err(InvokeError::from)
}

/// `speech_synthesize`: `{identity, text}` → raw MP3 via ipc::Response.
#[tauri::command(rename_all = "snake_case")]
pub async fn speech_synthesize(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
    identity: SpeechIdentity,
    text: String,
) -> Result<Response, InvokeError> {
    require_main_window(&window).map_err(InvokeError::from)?;
    let mp3 = state
        .service
        .synthesize(identity, text)
        .await
        .map_err(InvokeError::from)?;
    Ok(Response::new(mp3))
}

/// `speech_cancel`: `{request_id}` → `{request_id, status}`.
#[derive(Debug, Serialize)]
pub struct SpeechCancelReply {
    pub request_id: String,
    pub status: &'static str,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn speech_cancel(
    state: State<'_, Arc<SpeechState>>,
    window: WebviewWindow,
    request_id: String,
) -> Result<SpeechCancelReply, InvokeError> {
    (|| -> SpeechResult<SpeechCancelReply> {
        require_main_window(&window)?;
        Ok(SpeechCancelReply {
            status: state.service.cancel(&request_id),
            request_id,
        })
    })()
    .map_err(InvokeError::from)
}
