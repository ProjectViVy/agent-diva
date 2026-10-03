//! `diva.speech/v1` preferences: a closed schema, strict validation, and
//! atomic file persistence behind a monotonic revision CAS. Readback never
//! carries secrets — credentials live in `credentials.rs` slots and only
//! their presence is reported.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{SpeechCode, SpeechError, SpeechResult};

pub const SPEECH_SCHEMA: &str = "diva.speech/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeechProvider {
    SiliconFlow,
    MiniMax,
}

impl SpeechProvider {
    pub fn parse(value: &str) -> SpeechResult<Self> {
        match value {
            "siliconflow" => Ok(Self::SiliconFlow),
            "minimax" => Ok(Self::MiniMax),
            other => Err(SpeechError::new(
                SpeechCode::InvalidInput,
                format!("unknown provider {other:?}"),
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SiliconFlow => "siliconflow",
            Self::MiniMax => "minimax",
        }
    }
}

/// `"system"` — the only accepted bare-string reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemRef;

impl Serialize for SystemRef {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("system")
    }
}

impl<'de> Deserialize<'de> for SystemRef {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = String::deserialize(d)?;
        if value == "system" {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(
                "reference string must be \"system\"",
            ))
        }
    }
}

/// Strict reference union: `system` | `{voice_id}` | `{asset_id,transcript}`.
/// Any other shape fails to deserialize and is rejected, never dropped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SpeechReference {
    System(SystemRef),
    Reusable {
        voice_id: String,
    },
    Inline {
        asset_id: String,
        transcript: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SttPreferences {
    pub provider: SpeechProvider,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiliconFlowTts {
    pub base_url: String,
    pub model: String,
    pub voice: String,
    pub speed: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<SpeechReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MiniMaxTts {
    pub base_url: String,
    pub model: String,
    pub voice_id: String,
    pub speed: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsPreferences {
    pub provider: SpeechProvider,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub siliconflow: Option<SiliconFlowTts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimax: Option<MiniMaxTts>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechPreferences {
    pub stt: SttPreferences,
    pub tts: TtsPreferences,
    #[serde(default)]
    pub auto_read_replies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechConfigFile {
    pub schema: String,
    pub revision: u64,
    pub preferences: SpeechPreferences,
}

fn invalid_input(message: impl Into<String>) -> SpeechError {
    SpeechError::new(SpeechCode::InvalidInput, message)
}

/// Base URL rule from C2-4: http(s) origin only — no credentials in the
/// URL, no query, no fragment.
pub fn validate_base_url(field: &str, url: &str) -> SpeechResult<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.len() > 512 {
        return Err(invalid_input(format!(
            "{field}: empty or overlong base_url"
        )));
    }
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return Err(invalid_input(format!("{field}: base_url needs a scheme")));
    };
    if scheme != "https" && scheme != "http" {
        return Err(invalid_input(format!(
            "{field}: base_url scheme must be http(s)"
        )));
    }
    if rest.is_empty() {
        return Err(invalid_input(format!("{field}: base_url has no host")));
    }
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() {
        return Err(invalid_input(format!("{field}: base_url has no host")));
    }
    if authority.contains('@') {
        return Err(invalid_input(format!(
            "{field}: base_url must not carry credentials"
        )));
    }
    if trimmed.contains('#') || trimmed.contains('?') {
        return Err(invalid_input(format!(
            "{field}: base_url must not carry query/fragment"
        )));
    }
    Ok(())
}

fn validate_bounded_str(field: &str, value: &str, max: usize) -> SpeechResult<()> {
    if value.trim().is_empty() {
        return Err(invalid_input(format!("{field} is required")));
    }
    if value.len() > max {
        return Err(invalid_input(format!("{field} exceeds {max} bytes")));
    }
    Ok(())
}

/// Semantic validation beyond serde shape: provider block must exist for
/// the selected provider, numeric bounds hold, referenced assets exist.
/// `asset_exists` is supplied by the caller (the shell's AssetStore) so a
/// deleted/missing reference is an explicit error, not a stale pointer.
pub fn validate_preferences(
    prefs: &SpeechPreferences,
    asset_exists: &dyn Fn(&str) -> bool,
) -> SpeechResult<()> {
    if prefs.stt.provider != SpeechProvider::SiliconFlow {
        return Err(SpeechError::new(
            SpeechCode::UnsupportedReference,
            "stt.provider must be siliconflow",
        ));
    }
    validate_base_url("stt.base_url", &prefs.stt.base_url)?;
    validate_bounded_str("stt.model", &prefs.stt.model, 128)?;

    match prefs.tts.provider {
        SpeechProvider::SiliconFlow => {
            let Some(block) = &prefs.tts.siliconflow else {
                return Err(invalid_input(
                    "tts.provider=siliconflow requires a siliconflow block",
                ));
            };
            validate_base_url("tts.siliconflow.base_url", &block.base_url)?;
            validate_bounded_str("tts.siliconflow.model", &block.model, 128)?;
            validate_bounded_str("tts.siliconflow.voice", &block.voice, 256)?;
            if !(0.5..=2.0).contains(&block.speed) {
                return Err(invalid_input(
                    "tts.siliconflow.speed out of range 0.5..=2.0",
                ));
            }
            if let Some(reference) = &block.reference {
                validate_reference(reference, asset_exists)?;
            }
        }
        SpeechProvider::MiniMax => {
            let Some(block) = &prefs.tts.minimax else {
                return Err(invalid_input(
                    "tts.provider=minimax requires a minimax block",
                ));
            };
            validate_base_url("tts.minimax.base_url", &block.base_url)?;
            validate_bounded_str("tts.minimax.model", &block.model, 128)?;
            validate_bounded_str("tts.minimax.voice_id", &block.voice_id, 256)?;
            if !(0.5..=2.0).contains(&block.speed) {
                return Err(invalid_input("tts.minimax.speed out of range 0.5..=2.0"));
            }
            if !(0.0..=10.0).contains(&block.volume) {
                return Err(invalid_input("tts.minimax.volume out of range 0.0..=10.0"));
            }
        }
    }
    Ok(())
}

fn validate_reference(
    reference: &SpeechReference,
    asset_exists: &dyn Fn(&str) -> bool,
) -> SpeechResult<()> {
    match reference {
        SpeechReference::System(_) => Ok(()),
        SpeechReference::Reusable { voice_id } => {
            // SiliconFlow reusable uri shape: speech:<customName>:<id>
            if !voice_id.starts_with("speech:") || voice_id.len() > 256 {
                return Err(SpeechError::new(
                    SpeechCode::UnsupportedReference,
                    "reusable reference must be a speech:<name>:<id> uri",
                ));
            }
            Ok(())
        }
        SpeechReference::Inline {
            asset_id,
            transcript,
        } => {
            crate::assets::validate_asset_id(asset_id)?;
            if transcript.trim().is_empty() || transcript.len() > 4096 {
                return Err(SpeechError::new(
                    SpeechCode::UnsupportedReference,
                    "inline reference transcript empty or overlong",
                ));
            }
            if !asset_exists(asset_id) {
                return Err(SpeechError::new(
                    SpeechCode::AssetNotFound,
                    "configured reference asset is missing",
                ));
            }
            Ok(())
        }
    }
}

fn default_preferences() -> SpeechPreferences {
    SpeechPreferences {
        stt: SttPreferences {
            provider: SpeechProvider::SiliconFlow,
            base_url: "https://api.siliconflow.cn".into(),
            model: "FunAudioLLM/SenseVoiceSmall".into(),
        },
        tts: TtsPreferences {
            provider: SpeechProvider::SiliconFlow,
            siliconflow: Some(SiliconFlowTts {
                base_url: "https://api.siliconflow.cn".into(),
                model: "FunAudioLLM/CosyVoice2-0.5B".into(),
                voice: "FunAudioLLM/CosyVoice2-0.5B:alex".into(),
                speed: 1.0,
                reference: None,
            }),
            minimax: None,
        },
        auto_read_replies: false,
    }
}

/// Write `bytes` to `path` atomically (tmp file + rename) so a crash mid
/// write never leaves a torn preferences or manifest file.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> SpeechResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid_input("path has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|e| invalid_input(format!("create {}: {e}", parent.display())))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| {
        SpeechError::new(SpeechCode::Busy, format!("write {}: {e}", tmp.display())).retryable(true)
    })?;
    fs::rename(&tmp, path).map_err(|e| {
        SpeechError::new(SpeechCode::Busy, format!("replace {}: {e}", path.display()))
            .retryable(true)
    })
}

/// Versioned preferences file with revision CAS. One file, one writer —
/// the shell serializes access through a Mutex.
pub struct ConfigStore {
    path: PathBuf,
    file: SpeechConfigFile,
}

impl std::fmt::Debug for ConfigStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigStore")
            .field("path", &self.path)
            .field("revision", &self.file.revision)
            .finish()
    }
}

impl ConfigStore {
    /// Load or seed `path`. A missing file starts at revision 0 with
    /// default preferences; a corrupt file is an explicit error, never
    /// silently re-seeded over user data.
    pub fn open(path: PathBuf) -> SpeechResult<Self> {
        let file = if path.exists() {
            let text = fs::read_to_string(&path)
                .map_err(|e| invalid_input(format!("read {}: {e}", path.display())))?;
            let parsed: SpeechConfigFile = serde_json::from_str(&text)
                .map_err(|e| invalid_input(format!("preferences file is corrupt: {e}")))?;
            if parsed.schema != SPEECH_SCHEMA {
                return Err(invalid_input(format!(
                    "preferences schema {:?} != {SPEECH_SCHEMA:?}",
                    parsed.schema
                )));
            }
            parsed
        } else {
            SpeechConfigFile {
                schema: SPEECH_SCHEMA.into(),
                revision: 0,
                preferences: default_preferences(),
            }
        };
        Ok(Self { path, file })
    }

    pub fn revision(&self) -> u64 {
        self.file.revision
    }

    pub fn preferences(&self) -> &SpeechPreferences {
        &self.file.preferences
    }

    /// CAS update: `base_revision` must equal the current revision or the
    /// write is a conflict — never a last-writer-wins overwrite.
    pub fn update(
        &mut self,
        base_revision: u64,
        preferences: SpeechPreferences,
        asset_exists: &dyn Fn(&str) -> bool,
    ) -> SpeechResult<u64> {
        if base_revision != self.file.revision {
            return Err(SpeechError::new(
                SpeechCode::RevisionConflict,
                format!(
                    "preferences moved to revision {} — refetch before writing",
                    self.file.revision
                ),
            ));
        }
        validate_preferences(&preferences, asset_exists)?;
        let next = SpeechConfigFile {
            schema: SPEECH_SCHEMA.into(),
            revision: self.file.revision + 1,
            preferences,
        };
        let bytes = serde_json::to_vec_pretty(&next)
            .map_err(|e| invalid_input(format!("encode preferences: {e}")))?;
        atomic_write(&self.path, &bytes)?;
        self.file = next;
        Ok(self.file.revision)
    }
}
