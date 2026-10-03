//! DN-6B contract tests: admission fencing, cancellation races, byte
//! caps, provider error semantics, and quit teardown against a local
//! HTTP fixture server. `allow_insecure_loopback` is the test seam that
//! lets `http://127.0.0.1` endpoints exercise the lane; production
//! services never set it.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use diva_speech::assets::AssetStore;
use diva_speech::config::{
    ConfigStore, MiniMaxTts, SiliconFlowTts, SpeechPreferences, SpeechProvider,
};
use diva_speech::credentials::{CredentialStore, SecretStore};
use diva_speech::service::{Diagnostic, SpeechIdentity, SpeechService};
use diva_speech::{SpeechCode, SpeechError, SpeechResult};

// ---------- fixture HTTP server ----------

#[derive(Debug)]
struct Recorded {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Recorded {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

struct RawResponse {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl RawResponse {
    fn json(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "application/json",
            body: body.into().into_bytes(),
        }
    }
    fn bytes(status: u16, body: Vec<u8>) -> Self {
        Self {
            status,
            content_type: "audio/mpeg",
            body,
        }
    }
}

struct Fixture {
    base: String,
    requests: Arc<Mutex<Vec<Recorded>>>,
    stop: Arc<AtomicBool>,
    _join: JoinHandle<()>,
}

impl Fixture {
    /// Serve every accepted connection through `handler`.
    fn serve(handler: impl Fn(&Recorded) -> RawResponse + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let join = {
            let requests = requests.clone();
            let stop = stop.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream.set_nonblocking(false).unwrap();
                            let recorded = read_request(&mut stream);
                            requests.lock().unwrap().push(recorded.clone_shallow());
                            let resp = handler(&recorded);
                            write_response(&mut stream, resp);
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
            })
        };
        Self {
            base: format!("http://127.0.0.1:{port}"),
            requests,
            stop,
            _join: join,
        }
    }

    /// Blackhole: accept nothing. TCP connect completes in the kernel;
    /// the request sits unanswered until cancelled.
    fn blackhole() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let join = std::thread::spawn(move || {
            while !stop_thread.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(10));
            }
            drop(listener);
        });
        Self {
            base: format!("http://127.0.0.1:{port}"),
            requests,
            stop,
            _join: join,
        }
    }

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Recorded {
    fn clone_shallow(&self) -> Recorded {
        Recorded {
            method: self.method.clone(),
            path: self.path.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Recorded {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    let header_end = loop {
        let n = stream.read(&mut tmp).unwrap();
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            break pos + 4;
        }
        assert!(buf.len() < 1024 * 1024, "request headers too large");
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.lines();
    let mut first = lines.next().unwrap_or_default().split_whitespace();
    let method = first.next().unwrap_or_default().to_string();
    let path = first.next().unwrap_or_default().to_string();
    let mut headers = Vec::new();
    let mut content_len = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim().to_string();
            let v = v.trim().to_string();
            if k.eq_ignore_ascii_case("content-length") {
                content_len = v.parse().unwrap_or(0);
            }
            headers.push((k, v));
        }
    }
    while buf.len() < header_end + content_len {
        let n = stream.read(&mut tmp).unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    Recorded {
        method,
        path,
        headers,
        body: buf[header_end..].to_vec(),
    }
}

fn write_response(stream: &mut TcpStream, resp: RawResponse) {
    let head = format!(
        "HTTP/1.1 {} OK\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        resp.status,
        resp.content_type,
        resp.body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&resp.body);
    let _ = stream.flush();
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

// ---------- test plumbing ----------

struct MemSecrets {
    map: Mutex<HashMap<String, String>>,
    fail: bool,
}

impl MemSecrets {
    fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
            fail: false,
        }
    }
}

impl SecretStore for MemSecrets {
    fn set(&self, _service: &str, user: &str, secret: &str) -> SpeechResult<()> {
        if self.fail {
            return Err(SpeechError::new(
                SpeechCode::CredentialUnavailable,
                "store down",
            ));
        }
        self.map
            .lock()
            .unwrap()
            .insert(user.to_string(), secret.to_string());
        Ok(())
    }
    fn get(&self, _service: &str, user: &str) -> SpeechResult<Option<String>> {
        if self.fail {
            return Err(SpeechError::new(
                SpeechCode::CredentialUnavailable,
                "store down",
            ));
        }
        Ok(self.map.lock().unwrap().get(user).cloned())
    }
    fn delete(&self, _service: &str, user: &str) -> SpeechResult<()> {
        self.map.lock().unwrap().remove(user);
        Ok(())
    }
}

fn wav_16k_ms(ms: usize) -> Vec<u8> {
    let samples = ms * 16;
    let data_len = (samples * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&16000u32.to_le_bytes());
    out.extend_from_slice(&32000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend(std::iter::repeat_n(0u8, data_len as usize));
    out
}

fn stereo_wav() -> Vec<u8> {
    let mut w = wav_16k_ms(100);
    w[22] = 2; // channels = 2
    w
}

const FAKE_MP3: &[u8] = b"ID3\x04\x00\x00\x00\x00\x00\x21fake-frame-bytes-for-tests";

fn hex_upper(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

fn identity(req: &str, session: &str, gen: u64) -> SpeechIdentity {
    SpeechIdentity {
        request_id: req.to_string(),
        session_id: session.to_string(),
        run_id: None,
        utterance_id: format!("u-{req}"),
        generation: gen,
    }
}

struct Harness {
    service: SpeechService<MemSecrets>,
    diagnostics: Arc<Mutex<Vec<Diagnostic>>>,
    _tmp: tempfile::TempDir,
}

fn harness(base_url: &str, tts_provider: SpeechProvider) -> Harness {
    let tmp = tempfile::tempdir().unwrap();
    let diagnostics: Arc<Mutex<Vec<Diagnostic>>> = Arc::new(Mutex::new(Vec::new()));
    let sink_diags = diagnostics.clone();
    let config = ConfigStore::open(tmp.path().join("speech.json")).unwrap();
    let credentials = CredentialStore::with_store(MemSecrets::new());
    let assets = AssetStore::open(tmp.path().join("assets")).unwrap();
    let service = SpeechService::with_parts(
        config,
        credentials,
        assets,
        Arc::new(move |d| sink_diags.lock().unwrap().push(d)),
        true, // allow_insecure_loopback — fixture server is plain http
    )
    .unwrap();

    let prefs = SpeechPreferences {
        stt: diva_speech::config::SttPreferences {
            provider: SpeechProvider::SiliconFlow,
            base_url: base_url.to_string(),
            model: "FunAudioLLM/SenseVoiceSmall".into(),
        },
        tts: diva_speech::config::TtsPreferences {
            provider: tts_provider,
            siliconflow: Some(SiliconFlowTts {
                base_url: base_url.to_string(),
                model: "FunAudioLLM/CosyVoice2-0.5B".into(),
                voice: "FunAudioLLM/CosyVoice2-0.5B:alex".into(),
                speed: 1.0,
                reference: None,
            }),
            minimax: Some(MiniMaxTts {
                base_url: base_url.to_string(),
                model: "speech-2.8-hd".into(),
                voice_id: "male-qn-qingse".into(),
                speed: 1.0,
                volume: 5.0,
            }),
        },
        auto_read_replies: false,
    };
    service
        .config()
        .lock()
        .unwrap()
        .update(0, prefs, &|_| true)
        .unwrap();
    service
        .credentials()
        .set(SpeechProvider::SiliconFlow, "sf-test-key")
        .unwrap();
    service
        .credentials()
        .set(SpeechProvider::MiniMax, "mm-test-key")
        .unwrap();
    service.set_context("session-1", 1).unwrap();
    Harness {
        service,
        diagnostics,
        _tmp: tmp,
    }
}

fn diag_phases(h: &Harness) -> Vec<&'static str> {
    h.diagnostics
        .lock()
        .unwrap()
        .iter()
        .map(|d| d.phase)
        .collect()
}

// ---------- named DN-6B tests ----------

#[tokio::test]
async fn stale_context_cannot_admit() {
    let fx = Fixture::serve(|_| RawResponse::json(200, r#"{"text":"late"}"#));
    let h = harness(&fx.base, SpeechProvider::SiliconFlow);
    h.service.set_context("session-1", 2).unwrap();

    let err = h
        .service
        .transcribe(identity("r1", "session-1", 1), wav_16k_ms(500))
        .await
        .unwrap_err();
    assert_eq!(err.code, SpeechCode::StaleContext);
    assert_eq!(fx.request_count(), 0, "stale request must never hit HTTP");
    assert_eq!(h.service.active_requests(), 0);
}

#[tokio::test]
async fn cancel_before_http() {
    let fx = Fixture::blackhole();
    let h = harness(&fx.base, SpeechProvider::SiliconFlow);

    let svc = h.service.clone();
    let task = tokio::spawn(async move {
        svc.transcribe(identity("r2", "session-1", 1), wav_16k_ms(500))
            .await
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(h.service.cancel("r2"), "cancelled");
    let err = task.await.unwrap().unwrap_err();
    assert_eq!(err.code, SpeechCode::Cancelled);
    assert_eq!(fx.request_count(), 0, "blackhole server accepted nothing");
    // second cancel is idempotent
    assert_eq!(h.service.cancel("r2"), "settled");
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(h.service.active_requests(), 0);
    assert!(diag_phases(&h).contains(&"cancelled"));
}

#[tokio::test]
async fn late_completion_discarded() {
    let fx = Fixture::serve(|_| RawResponse::json(200, r#"{"text":"done"}"#));
    let h = harness(&fx.base, SpeechProvider::SiliconFlow);
    // The provider responds, then the context advances before settle:
    // the result must be discarded as stale_context.
    let svc = h.service.clone();
    h.service.set_pre_settle_hook(Some(Arc::new(move || {
        let _ = svc.set_context("session-1", 2);
    })));

    let err = h
        .service
        .transcribe(identity("r3", "session-1", 1), wav_16k_ms(300))
        .await
        .unwrap_err();
    assert_eq!(err.code, SpeechCode::StaleContext);
    assert_eq!(fx.request_count(), 1, "provider did answer — but too late");
    assert_eq!(h.service.active_requests(), 0);
    assert!(diag_phases(&h).contains(&"discarded"));
}

#[tokio::test]
async fn oversize_body_bounded() {
    let big = vec![0xFFu8; 17 * 1024 * 1024];
    let fx = Fixture::serve(move |_| RawResponse::bytes(200, big.clone()));
    let h = harness(&fx.base, SpeechProvider::SiliconFlow);

    let err = h
        .service
        .synthesize(identity("r4", "session-1", 1), "hello".to_string())
        .await
        .unwrap_err();
    assert_eq!(err.code, SpeechCode::ProviderError);
    assert!(err.message.contains("cap"));
    assert_eq!(h.service.active_requests(), 0);
}

#[tokio::test]
async fn minimax_business_error() {
    // HTTP 200 with base_resp.status_code != 0 is an error, never audio.
    let fx = Fixture::serve(|_| {
        RawResponse::json(
            200,
            r#"{"base_resp":{"status_code":1004,"status_msg":"no permission"},"trace_id":"t1"}"#,
        )
    });
    let h = harness(&fx.base, SpeechProvider::MiniMax);

    let err = h
        .service
        .synthesize(identity("r5", "session-1", 1), "你好".to_string())
        .await
        .unwrap_err();
    assert_eq!(err.code, SpeechCode::ProviderError);
    assert_eq!(err.http_status, Some(200));
    assert_eq!(err.provider, Some(SpeechProvider::MiniMax));
    assert_eq!(h.service.active_requests(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quit_with_inflight() {
    let fx = Fixture::blackhole();
    let h = harness(&fx.base, SpeechProvider::SiliconFlow);

    let svc = h.service.clone();
    let task = tokio::spawn(async move {
        svc.transcribe(identity("r6", "session-1", 1), wav_16k_ms(800))
            .await
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(h.service.active_requests(), 1);

    let report = h.service.shutdown(Duration::from_secs(5)).await;
    assert_eq!(report.inflight_at_start, 1);
    assert_eq!(
        report.remaining, 0,
        "in-flight request must join inside grace"
    );
    assert_eq!(h.service.active_requests(), 0);

    let err = task.await.unwrap().unwrap_err();
    assert_eq!(err.code, SpeechCode::Cancelled);

    // admission after quit is rejected
    let err = h
        .service
        .transcribe(identity("r7", "session-1", 1), wav_16k_ms(100))
        .await
        .unwrap_err();
    assert_eq!(err.code, SpeechCode::Cancelled);
    assert_eq!(fx.request_count(), 0);
}

// ---------- supporting coverage ----------

#[tokio::test]
async fn siliconflow_stt_happy() {
    let fx = Fixture::serve(|req| {
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/v1/audio/transcriptions");
        let ct = req.header("content-type").unwrap();
        assert!(ct.starts_with("multipart/form-data"));
        let body = String::from_utf8_lossy(&req.body);
        assert!(body.contains("filename=\"audio.wav\""));
        assert!(body.contains("FunAudioLLM/SenseVoiceSmall"));
        assert_eq!(req.header("authorization").unwrap(), "Bearer sf-test-key");
        RawResponse::json(200, r#"{"text":"你好世界"}"#)
    });
    let h = harness(&fx.base, SpeechProvider::SiliconFlow);

    let reply = h
        .service
        .transcribe(identity("r8", "session-1", 1), wav_16k_ms(400))
        .await
        .unwrap();
    assert_eq!(reply.status, "transcribed");
    assert_eq!(reply.text, "你好世界");
    assert_eq!(reply.identity.request_id, "r8");
    assert_eq!(fx.request_count(), 1);
    assert_eq!(h.service.active_requests(), 0);
    assert!(diag_phases(&h).contains(&"settled"));
}

#[tokio::test]
async fn siliconflow_tts_happy() {
    let fx = Fixture::serve(|req| {
        assert_eq!(req.path, "/v1/audio/speech");
        let json: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        assert_eq!(json["model"], "FunAudioLLM/CosyVoice2-0.5B");
        assert_eq!(json["voice"], "FunAudioLLM/CosyVoice2-0.5B:alex");
        assert_eq!(json["response_format"], "mp3");
        assert_eq!(json["stream"], false);
        assert!(json.get("references").is_none());
        RawResponse::bytes(200, FAKE_MP3.to_vec())
    });
    let h = harness(&fx.base, SpeechProvider::SiliconFlow);

    let mp3 = h
        .service
        .synthesize(identity("r9", "session-1", 1), "read this".to_string())
        .await
        .unwrap();
    assert_eq!(mp3, FAKE_MP3);
    assert_eq!(h.service.active_requests(), 0);
}

#[tokio::test]
async fn minimax_tts_happy() {
    let fx = Fixture::serve(|req| {
        assert_eq!(req.path, "/v1/t2a_v2");
        let json: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        assert_eq!(json["model"], "speech-2.8-hd");
        assert_eq!(json["voice_setting"]["voice_id"], "male-qn-qingse");
        assert_eq!(json["audio_setting"]["format"], "mp3");
        RawResponse::json(
            200,
            format!(
                r#"{{"data":{{"audio":"{}","status":2}},"base_resp":{{"status_code":0,"status_msg":"success"}},"trace_id":"t2"}}"#,
                hex_upper(FAKE_MP3)
            ),
        )
    });
    let h = harness(&fx.base, SpeechProvider::MiniMax);

    let mp3 = h
        .service
        .synthesize(identity("r10", "session-1", 1), "你好".to_string())
        .await
        .unwrap();
    assert_eq!(mp3, FAKE_MP3);
}

#[tokio::test]
async fn wav_rejected_before_upload() {
    let fx = Fixture::serve(|_| panic!("no request may arrive"));
    let h = harness(&fx.base, SpeechProvider::SiliconFlow);
    let err = h
        .service
        .transcribe(identity("r11", "session-1", 1), stereo_wav())
        .await
        .unwrap_err();
    assert_eq!(err.code, SpeechCode::InvalidAudio);
    assert_eq!(fx.request_count(), 0);
    assert_eq!(h.service.active_requests(), 0);
}

#[tokio::test]
async fn context_idempotent_and_conflicting() {
    let fx = Fixture::blackhole();
    let h = harness(&fx.base, SpeechProvider::SiliconFlow);
    let ctx = h.service.set_context("session-1", 1).unwrap();
    assert_eq!(ctx.generation, 1);
    let err = h.service.set_context("session-9", 1).unwrap_err();
    assert_eq!(err.code, SpeechCode::StaleContext);
    let err = h.service.set_context("session-1", 0).unwrap_err();
    assert_eq!(err.code, SpeechCode::StaleContext);
}
