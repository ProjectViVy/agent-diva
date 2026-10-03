//! DN-6B request lane: admission (trusted window + session + monotonic
//! generation, one STT and one TTS slot), frozen config/credential/
//! reference snapshots taken at admission, bounded provider HTTP on the
//! caller's tokio runtime, and abort/join cleanup on cancel, context
//! advance, hide and quit. The registry mutex never spans keyring or
//! HTTP work.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::assets::{AssetLease, AssetStore};
use crate::config::{
    ConfigStore, MiniMaxTts, SiliconFlowTts, SpeechProvider, SpeechReference, SttPreferences,
};
use crate::credentials::{CredentialStore, OsKeyring, SecretStore};
use crate::providers;
use crate::registry::{CancelToken, Registry, RequestKind, SpeechContext};
use crate::{wav, SpeechCode, SpeechError, SpeechResult};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(120);
const META_ID_MAX: usize = 64;
const META_SESSION_MAX: usize = 128;
const META_FIELD_MAX: usize = 128;

/// C2-4 `SpeechIdentity`: correlation and fencing fields — never a VIVY
/// authority grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechIdentity {
    pub request_id: String,
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub utterance_id: String,
    pub generation: u64,
}

fn validate_id(field: &str, value: &str, max: usize) -> SpeechResult<()> {
    if value.is_empty() || value.len() > max {
        return Err(SpeechError::new(
            SpeechCode::InvalidInput,
            format!("{field} empty or overlong"),
        ));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':'))
    {
        return Err(SpeechError::new(
            SpeechCode::InvalidInput,
            format!("{field} contains unsafe characters"),
        ));
    }
    Ok(())
}

impl SpeechIdentity {
    pub fn validate(&self) -> SpeechResult<()> {
        validate_id("request_id", &self.request_id, META_ID_MAX)?;
        validate_id("session_id", &self.session_id, META_SESSION_MAX)?;
        validate_id("utterance_id", &self.utterance_id, META_FIELD_MAX)?;
        if let Some(run) = &self.run_id {
            validate_id("run_id", run, META_FIELD_MAX)?;
        }
        Ok(())
    }
}

/// Safe `speech:diagnostic` payload: phase/provider/safe code/HTTP
/// status/elapsed/byte counts/identity. No secrets, audio, or text.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub phase: &'static str,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SpeechProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<SpeechCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    pub elapsed_ms: u64,
    pub bytes: u64,
    pub request_id: String,
    pub utterance_id: String,
    pub generation: u64,
    pub config_revision: u64,
}

/// `speech_transcribe` success shape — identity echoed back.
#[derive(Debug, Clone, Serialize)]
pub struct TranscribeReply {
    pub identity: SpeechIdentity,
    pub status: &'static str,
    pub text: String,
}

/// Honest teardown accounting for `quit_with_inflight`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownReport {
    pub inflight_at_start: usize,
    pub joined: usize,
    pub remaining: usize,
}

/// Frozen request snapshot taken at admission: nothing in here can be
/// altered by a later config/credential/reference change.
enum Prepared {
    Stt {
        cfg: SttPreferences,
        key: String,
        wav: Vec<u8>,
    },
    SiliconFlowTts {
        cfg: SiliconFlowTts,
        key: String,
        text: String,
        inline: Option<AssetLease>,
    },
    MiniMaxTts {
        cfg: MiniMaxTts,
        key: String,
        text: String,
    },
}

struct ServiceInner<S: SecretStore> {
    config: Mutex<ConfigStore>,
    credentials: CredentialStore<S>,
    assets: AssetStore,
    registry: Registry,
    client: reqwest::Client,
    allow_insecure_loopback: bool,
    diagnostic: Arc<dyn Fn(Diagnostic) + Send + Sync>,
    /// Test seam: runs between the provider result and registry settle,
    /// letting a fixture make a completion provably late.
    #[doc(hidden)]
    pre_settle_hook: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

pub struct SpeechService<S: SecretStore = OsKeyring> {
    inner: Arc<ServiceInner<S>>,
}

impl<S: SecretStore> Clone for SpeechService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

fn make_client() -> SpeechResult<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| {
            SpeechError::new(
                SpeechCode::NativeUnavailable,
                format!("http client init failed: {e}"),
            )
        })
}

impl SpeechService<OsKeyring> {
    /// Production open: OS keyring, on-disk config/assets under `dir`.
    /// `diagnostic` receives every emitted `speech:diagnostic` payload.
    pub fn open(
        dir: PathBuf,
        diagnostic: Arc<dyn Fn(Diagnostic) + Send + Sync>,
    ) -> SpeechResult<Self> {
        Self::with_parts(
            ConfigStore::open(dir.join("speech.json"))?,
            CredentialStore::os(),
            AssetStore::open(dir.join("assets"))?,
            diagnostic,
            false,
        )
    }
}

impl<S: SecretStore + 'static> SpeechService<S> {
    pub fn with_parts(
        config: ConfigStore,
        credentials: CredentialStore<S>,
        assets: AssetStore,
        diagnostic: Arc<dyn Fn(Diagnostic) + Send + Sync>,
        allow_insecure_loopback: bool,
    ) -> SpeechResult<Self> {
        Ok(Self {
            inner: Arc::new(ServiceInner {
                config: Mutex::new(config),
                credentials,
                assets,
                registry: Registry::default(),
                client: make_client()?,
                allow_insecure_loopback,
                diagnostic,
                pre_settle_hook: Mutex::new(None),
            }),
        })
    }

    pub fn config(&self) -> &Mutex<ConfigStore> {
        &self.inner.config
    }

    pub fn credentials(&self) -> &CredentialStore<S> {
        &self.inner.credentials
    }

    pub fn assets(&self) -> &AssetStore {
        &self.inner.assets
    }

    pub fn context(&self) -> Option<SpeechContext> {
        self.inner.registry.context()
    }

    pub fn active_requests(&self) -> usize {
        self.inner.registry.active_count()
    }

    #[doc(hidden)]
    pub fn set_pre_settle_hook(&self, hook: Option<Arc<dyn Fn() + Send + Sync>>) {
        if let Ok(mut slot) = self.inner.pre_settle_hook.lock() {
            *slot = hook;
        }
    }

    /// `speech_context_set`.
    pub fn set_context(&self, session_id: &str, generation: u64) -> SpeechResult<SpeechContext> {
        validate_id("session_id", session_id, META_SESSION_MAX)?;
        self.inner.registry.set_context(session_id, generation)
    }

    /// Hide/close invalidation: drop the context and abort everything.
    pub fn invalidate_context(&self) {
        self.inner.registry.invalidate_context();
    }

    /// `speech_cancel` — idempotent, window-scoped.
    pub fn cancel(&self, request_id: &str) -> &'static str {
        self.inner.registry.cancel(request_id)
    }

    fn admit_common(
        &self,
        identity: &SpeechIdentity,
        kind: RequestKind,
    ) -> SpeechResult<(Arc<CancelToken>, Instant)> {
        identity.validate()?;
        let token = self.inner.registry.admit(
            &identity.request_id,
            &identity.session_id,
            identity.generation,
            kind,
        )?;
        Ok((token, Instant::now()))
    }

    /// Snapshot config + credential + (for inline references) the asset
    /// lease, OUTSIDE the registry lock. On failure the reservation is
    /// released so slots return to zero on every terminal path.
    /// `wav`/`text` move into the snapshot so the admitted request carries
    /// its own payload end to end.
    fn prepare(
        &self,
        identity: &SpeechIdentity,
        kind: RequestKind,
        wav: Vec<u8>,
        text: String,
    ) -> SpeechResult<Prepared> {
        let cleanup = |e: SpeechError| -> SpeechError {
            self.inner.registry.settle(&identity.request_id);
            e
        };
        let prefs = self
            .inner
            .config
            .lock()
            .map_err(|_| cleanup(SpeechError::new(SpeechCode::Busy, "config lock poisoned")))?
            .preferences()
            .clone();
        match kind {
            RequestKind::Stt => {
                let key = self
                    .inner
                    .credentials
                    .get_secret(prefs.stt.provider)
                    .map_err(&cleanup)?
                    .ok_or_else(|| {
                        cleanup(SpeechError::new(
                            SpeechCode::NotConfigured,
                            "no STT provider credential configured",
                        ))
                    })?;
                Ok(Prepared::Stt {
                    cfg: prefs.stt.clone(),
                    key,
                    wav,
                })
            }
            RequestKind::Tts => match prefs.tts.provider {
                SpeechProvider::SiliconFlow => {
                    let cfg = prefs.tts.siliconflow.clone().ok_or_else(|| {
                        cleanup(SpeechError::new(
                            SpeechCode::NotConfigured,
                            "siliconflow TTS block not configured",
                        ))
                    })?;
                    let key = self
                        .inner
                        .credentials
                        .get_secret(SpeechProvider::SiliconFlow)
                        .map_err(&cleanup)?
                        .ok_or_else(|| {
                            cleanup(SpeechError::new(
                                SpeechCode::NotConfigured,
                                "no SiliconFlow credential configured",
                            ))
                        })?;
                    let inline = match &cfg.reference {
                        Some(SpeechReference::Inline { asset_id, .. }) => {
                            Some(self.inner.assets.read(asset_id).map_err(&cleanup)?)
                        }
                        _ => None,
                    };
                    Ok(Prepared::SiliconFlowTts {
                        cfg,
                        key,
                        text,
                        inline,
                    })
                }
                SpeechProvider::MiniMax => {
                    let cfg = prefs.tts.minimax.clone().ok_or_else(|| {
                        cleanup(SpeechError::new(
                            SpeechCode::NotConfigured,
                            "minimax TTS block not configured",
                        ))
                    })?;
                    let key = self
                        .inner
                        .credentials
                        .get_secret(SpeechProvider::MiniMax)
                        .map_err(&cleanup)?
                        .ok_or_else(|| {
                            cleanup(SpeechError::new(
                                SpeechCode::NotConfigured,
                                "no MiniMax credential configured",
                            ))
                        })?;
                    Ok(Prepared::MiniMaxTts { cfg, key, text })
                }
            },
        }
    }

    fn config_revision(&self) -> u64 {
        self.inner.config.lock().map(|c| c.revision()).unwrap_or(0)
    }

    fn diagnose(&self, phase: &'static str, report: DiagReport<'_>) {
        (self.inner.diagnostic)(Diagnostic {
            phase,
            kind: report.kind.as_str(),
            provider: report.provider,
            code: report.err.map(|e| e.code),
            http_status: report.err.and_then(|e| e.http_status),
            elapsed_ms: report.started.elapsed().as_millis() as u64,
            bytes: report.bytes,
            request_id: report.identity.request_id.clone(),
            utterance_id: report.identity.utterance_id.clone(),
            generation: report.identity.generation,
            config_revision: self.config_revision(),
        });
    }

    /// Spawn the request task. The task owns provider I/O, the pre-settle
    /// hook, registry settle and diagnostics; the caller awaits the
    /// outcome channel.
    fn spawn_request(
        &self,
        kind: RequestKind,
        identity: SpeechIdentity,
        prepared: Prepared,
        token: Arc<CancelToken>,
        started: Instant,
    ) -> oneshot::Receiver<SpeechResult<PreparedOutcome>> {
        let inner = self.inner.clone();
        let request_id = identity.request_id.clone();
        let (tx, rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            let outcome = inner.execute(prepared, &token).await;
            if let Ok(guard) = inner.pre_settle_hook.lock() {
                if let Some(hook) = guard.as_ref() {
                    hook();
                }
            }
            let deliverable = inner.registry.settle(&request_id);
            let outcome = match outcome {
                Ok(_o) if !deliverable => Err(SpeechError::new(
                    SpeechCode::StaleContext,
                    "request settled after its context advanced — result discarded",
                )),
                o => o,
            };
            let service = SpeechService::<S> { inner };
            let (phase, bytes, provider, err) = match &outcome {
                Ok(o) => ("settled", o.bytes, o.provider, None),
                Err(e) if e.code == SpeechCode::Cancelled => ("cancelled", 0, e.provider, Some(e)),
                Err(e) if e.code == SpeechCode::StaleContext && !deliverable => {
                    ("discarded", 0, e.provider, Some(e))
                }
                Err(e) => ("failed", 0, e.provider, Some(e)),
            };
            service.diagnose(
                phase,
                DiagReport {
                    kind,
                    provider,
                    started,
                    bytes,
                    identity: &identity,
                    err,
                },
            );
            let _ = tx.send(outcome);
        });
        self.inner.registry.track(handle);
        rx
    }

    /// `speech_transcribe`: Raw WAV body validated then uploaded.
    pub async fn transcribe(
        &self,
        identity: SpeechIdentity,
        wav_bytes: Vec<u8>,
    ) -> SpeechResult<TranscribeReply> {
        wav::validate_wav(&wav_bytes)?;
        let (token, started) = self.admit_common(&identity, RequestKind::Stt)?;
        // keyring/asset reads block briefly; keep them off the executor.
        let this = self.clone();
        let prep_identity = identity.clone();
        let prepared = tokio::task::spawn_blocking(move || {
            this.prepare(&prep_identity, RequestKind::Stt, wav_bytes, String::new())
        })
        .await
        .map_err(|_| SpeechError::new(SpeechCode::Busy, "prepare worker dropped"))??;
        self.diagnose(
            "admitted",
            DiagReport {
                kind: RequestKind::Stt,
                provider: Some(prepared.provider()),
                started,
                bytes: 0,
                identity: &identity,
                err: None,
            },
        );
        let rx = self.spawn_request(RequestKind::Stt, identity.clone(), prepared, token, started);
        let outcome = rx.await.map_err(|_| {
            SpeechError::new(SpeechCode::Busy, "request task dropped before replying")
        })??;
        let text = match outcome.payload {
            OutcomePayload::SttText(text) => text,
            OutcomePayload::Mp3(_) => unreachable!("stt outcome carried mp3"),
        };
        let status = if text.trim().is_empty() {
            "no_speech"
        } else {
            "transcribed"
        };
        Ok(TranscribeReply {
            identity,
            status,
            text: if status == "no_speech" {
                String::new()
            } else {
                text
            },
        })
    }

    /// `speech_synthesize`: JSON in, bounded MP3 bytes out.
    pub async fn synthesize(
        &self,
        identity: SpeechIdentity,
        text: String,
    ) -> SpeechResult<Vec<u8>> {
        if text.is_empty()
            || text.chars().count() > providers::TTS_TEXT_MAX_CHARS
            || text.len() > providers::TTS_TEXT_MAX_BYTES
        {
            return Err(SpeechError::new(
                SpeechCode::InvalidInput,
                "text empty or exceeds TTS bounds",
            ));
        }
        let (token, started) = self.admit_common(&identity, RequestKind::Tts)?;
        let this = self.clone();
        let prep_identity = identity.clone();
        let prepared = tokio::task::spawn_blocking(move || {
            this.prepare(&prep_identity, RequestKind::Tts, Vec::new(), text)
        })
        .await
        .map_err(|_| SpeechError::new(SpeechCode::Busy, "prepare worker dropped"))??;
        self.diagnose(
            "admitted",
            DiagReport {
                kind: RequestKind::Tts,
                provider: Some(prepared.provider()),
                started,
                bytes: 0,
                identity: &identity,
                err: None,
            },
        );
        let rx = self.spawn_request(RequestKind::Tts, identity, prepared, token, started);
        let outcome = rx.await.map_err(|_| {
            SpeechError::new(SpeechCode::Busy, "request task dropped before replying")
        })??;
        match outcome.payload {
            OutcomePayload::Mp3(mp3) => Ok(mp3),
            OutcomePayload::SttText(_) => unreachable!("tts outcome carried text"),
        }
    }

    /// Quit: reject admission, abort in-flight requests and join their
    /// tasks inside `grace`. The report states honestly what could not
    /// be joined.
    pub async fn shutdown(&self, grace: Duration) -> ShutdownReport {
        self.inner.registry.begin_shutdown();
        let (inflight, mut handles) = self.inner.registry.take_handles();
        let deadline = tokio::time::Instant::now() + grace;
        let mut joined = 0usize;
        for handle in &mut handles {
            match tokio::time::timeout_at(deadline, handle).await {
                Ok(_) => joined += 1,
                Err(_) => break,
            }
        }
        let remaining = handles.len() - joined;
        for handle in &handles[joined..] {
            handle.abort();
        }
        ShutdownReport {
            inflight_at_start: inflight,
            joined,
            remaining,
        }
    }
}

/// Bundle for `diagnose` — keeps the emitted fields grouped.
struct DiagReport<'a> {
    kind: RequestKind,
    provider: Option<SpeechProvider>,
    started: Instant,
    bytes: u64,
    identity: &'a SpeechIdentity,
    err: Option<&'a SpeechError>,
}

/// Per-request result: payload plus safe accounting for diagnostics.
pub struct PreparedOutcome {
    pub payload: OutcomePayload,
    pub bytes: u64,
    pub provider: Option<SpeechProvider>,
}

pub enum OutcomePayload {
    SttText(String),
    Mp3(Vec<u8>),
}

impl Prepared {
    fn provider(&self) -> SpeechProvider {
        match self {
            Self::Stt { cfg, .. } => cfg.provider,
            Self::SiliconFlowTts { .. } => SpeechProvider::SiliconFlow,
            Self::MiniMaxTts { .. } => SpeechProvider::MiniMax,
        }
    }
}

impl<S: SecretStore + 'static> ServiceInner<S> {
    async fn execute(
        &self,
        prepared: Prepared,
        token: &Arc<CancelToken>,
    ) -> SpeechResult<PreparedOutcome> {
        let work = self.provider_call(prepared);
        tokio::select! {
            _ = token.cancelled() => Err(SpeechError::new(
                SpeechCode::Cancelled,
                "request cancelled",
            )),
            result = work => result,
        }
    }

    async fn provider_call(&self, prepared: Prepared) -> SpeechResult<PreparedOutcome> {
        match prepared {
            Prepared::Stt { cfg, key, wav } => {
                let (text, bytes) = providers::siliconflow_stt(
                    &self.client,
                    &cfg,
                    &key,
                    wav,
                    self.allow_insecure_loopback,
                )
                .await?;
                Ok(PreparedOutcome {
                    payload: OutcomePayload::SttText(text),
                    bytes,
                    provider: Some(SpeechProvider::SiliconFlow),
                })
            }
            Prepared::SiliconFlowTts {
                cfg,
                key,
                text,
                inline,
            } => {
                let (mp3, bytes) = providers::siliconflow_tts(
                    &self.client,
                    &cfg,
                    &key,
                    &text,
                    inline.as_ref(),
                    self.allow_insecure_loopback,
                )
                .await?;
                Ok(PreparedOutcome {
                    payload: OutcomePayload::Mp3(mp3),
                    bytes,
                    provider: Some(SpeechProvider::SiliconFlow),
                })
            }
            Prepared::MiniMaxTts { cfg, key, text } => {
                let (mp3, bytes) = providers::minimax_tts(
                    &self.client,
                    &cfg,
                    &key,
                    &text,
                    self.allow_insecure_loopback,
                )
                .await?;
                Ok(PreparedOutcome {
                    payload: OutcomePayload::Mp3(mp3),
                    bytes,
                    provider: Some(SpeechProvider::MiniMax),
                })
            }
        }
    }
}
