use futures::{SinkExt, StreamExt};
use http::header::{HeaderValue, AUTHORIZATION};
use native_tls::TlsConnector;
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tokio::time::timeout;
use tokio_tungstenite::connect_async_tls_with_config;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};

type BoxError = Box<dyn Error + Send + Sync>;

const DEFAULT_BASE_URL: &str = "https://api.minimaxi.com";
const DEFAULT_MODEL: &str = "speech-2.8-hd";
const DEFAULT_VOICE_ID: &str = "male-qn-qingse";
const DEFAULT_OUTPUT_STEM: &str = "minimax_sync_tts_output";
const DEFAULT_FORMAT: &str = "mp3";

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let text = resolve_text()?;
    let api_key = required_env("MINIMAX_API_KEY")?;
    let base_url = env::var("MINIMAX_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let model = env::var("MINIMAX_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let voice_id = env::var("MINIMAX_VOICE_ID").unwrap_or_else(|_| DEFAULT_VOICE_ID.to_string());
    let audio_format =
        env::var("MINIMAX_AUDIO_FORMAT").unwrap_or_else(|_| DEFAULT_FORMAT.to_string());
    let output_path = resolve_output_path(&audio_format);

    println!("MiniMax sync TTS demo started");
    println!("base_url     : {}", base_url);
    println!("model        : {}", model);
    println!("voice_id     : {}", voice_id);
    println!("audio_format : {}", audio_format);
    println!("output_path  : {}", output_path.display());
    println!("text_length  : {}", text.chars().count());
    println!("tls_verify   : disabled");

    let mut socket = establish_connection(&base_url, &api_key).await?;
    let result = synthesize_sync(&mut socket, &model, &voice_id, &audio_format, &text).await?;
    finish_socket(&mut socket).await?;

    fs::write(&output_path, &result.audio_bytes).await?;
    println!("chunk_count  : {}", result.chunk_count);
    println!("audio_bytes  : {}", result.audio_bytes.len());
    println!(
        "trace_id     : {}",
        result.trace_id.unwrap_or_else(|| "n/a".to_string())
    );
    println!("saved audio  : {}", output_path.display());
    println!("done");
    Ok(())
}

fn resolve_text() -> Result<String, BoxError> {
    let cli_text = env::args().nth(1).unwrap_or_default();
    if !cli_text.trim().is_empty() {
        return Ok(cli_text);
    }

    let env_text = env::var("MINIMAX_TEXT").unwrap_or_default();
    if !env_text.trim().is_empty() {
        return Ok(env_text);
    }

    Err("missing text: pass it as the first CLI argument or set MINIMAX_TEXT".into())
}

fn required_env(key: &str) -> Result<String, BoxError> {
    env::var(key).map_err(|_| format!("missing required environment variable: {}", key).into())
}

fn resolve_output_path(output_format: &str) -> PathBuf {
    match env::var("MINIMAX_OUTPUT_PATH") {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => PathBuf::from(format!("{}.{}", DEFAULT_OUTPUT_STEM, output_format)),
    }
}

type MiniMaxSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn establish_connection(base_url: &str, api_key: &str) -> Result<MiniMaxSocket, BoxError> {
    let ws_url = minimax_websocket_url(base_url);
    let mut request = ws_url.clone().into_client_request()?;
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );

    let tls = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()?;
    let connector = Connector::NativeTls(tls.into());

    let (mut socket, _) = timeout(
        Duration::from_secs(10),
        connect_async_tls_with_config(request, None, false, Some(connector)),
    )
    .await
    .map_err(|_| {
        format!(
            "timed out while connecting to MiniMax websocket: {}",
            ws_url
        )
    })??;

    expect_event(&mut socket, "connected_success").await?;
    println!("connection   : connected_success");
    Ok(socket)
}

async fn synthesize_sync(
    socket: &mut MiniMaxSocket,
    model: &str,
    voice_id: &str,
    audio_format: &str,
    text: &str,
) -> Result<SynthesizeResult, BoxError> {
    send_json(
        socket,
        json!({
            "event": "task_start",
            "model": model,
            "voice_setting": {
                "voice_id": voice_id,
                "speed": 1,
                "vol": 1,
                "pitch": 0,
                "english_normalization": false
            },
            "audio_setting": {
                "sample_rate": 32000,
                "bitrate": 128000,
                "format": audio_format,
                "channel": 1
            }
        }),
    )
    .await?;
    expect_event(socket, "task_started").await?;
    println!("task_start   : task_started");

    send_json(
        socket,
        json!({
            "event": "task_continue",
            "text": text
        }),
    )
    .await?;

    let mut audio_bytes = Vec::new();
    let mut chunk_count = 0usize;
    let mut trace_id = None;

    loop {
        let message = read_json_message(socket).await?;
        if trace_id.is_none() {
            trace_id = message
                .get("trace_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }

        if let Some(audio_hex) = message
            .get("data")
            .and_then(|value| value.get("audio"))
            .and_then(Value::as_str)
        {
            if !audio_hex.trim().is_empty() {
                let chunk = decode_hex_audio(audio_hex)?;
                chunk_count += 1;
                println!("chunk        : #{} ({} bytes)", chunk_count, chunk.len());
                audio_bytes.extend_from_slice(&chunk);
            }
        }

        if let Some(event) = message.get("event").and_then(Value::as_str) {
            if event.eq_ignore_ascii_case("error") || event.eq_ignore_ascii_case("task_failed") {
                let detail = message
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("MiniMax websocket returned an error event");
                return Err(detail.to_string().into());
            }
        }

        if message
            .get("is_final")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            println!("is_final     : true");
            break;
        }
    }

    if audio_bytes.is_empty() {
        return Err("MiniMax sync TTS completed without audio data".into());
    }

    Ok(SynthesizeResult {
        audio_bytes,
        chunk_count,
        trace_id,
    })
}

async fn send_json(socket: &mut MiniMaxSocket, payload: Value) -> Result<(), BoxError> {
    socket
        .send(Message::Text(payload.to_string()))
        .await
        .map_err(|error| format!("failed to send MiniMax websocket message: {}", error).into())
}

async fn expect_event(socket: &mut MiniMaxSocket, expected: &str) -> Result<(), BoxError> {
    let message = read_json_message(socket).await?;
    let actual = message
        .get("event")
        .and_then(Value::as_str)
        .ok_or_else(|| "MiniMax websocket response is missing event field".to_string())?;

    if actual != expected {
        return Err(format!(
            "unexpected MiniMax websocket event: expected {}, got {}",
            expected, actual
        )
        .into());
    }

    Ok(())
}

async fn read_json_message(socket: &mut MiniMaxSocket) -> Result<Value, BoxError> {
    loop {
        let frame = timeout(Duration::from_secs(30), socket.next())
            .await
            .map_err(|_| "timed out while waiting for MiniMax websocket message".to_string())?
            .ok_or_else(|| "MiniMax websocket closed unexpectedly".to_string())??;

        match frame {
            Message::Text(text) => {
                let value = serde_json::from_str::<Value>(&text)
                    .map_err(|error| format!("invalid MiniMax websocket payload: {}", error))?;
                return Ok(value);
            }
            Message::Ping(payload) => {
                socket.send(Message::Pong(payload)).await?;
            }
            Message::Close(frame) => {
                let detail = frame
                    .map(|value| value.reason.to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "unknown close frame".to_string());
                return Err(format!("MiniMax websocket closed: {}", detail).into());
            }
            Message::Binary(_) => {
                return Err("MiniMax websocket returned unexpected binary frame".into());
            }
            Message::Pong(_) | Message::Frame(_) => {}
        }
    }
}

async fn finish_socket(socket: &mut MiniMaxSocket) -> Result<(), BoxError> {
    let _ = send_json(socket, json!({ "event": "task_finish" })).await;
    socket.close(None).await?;
    Ok(())
}

fn minimax_websocket_url(base_url: &str) -> String {
    let trimmed = trim_trailing_slash(base_url).trim_end_matches("/v1");
    let url = format!("{}/ws/v1/t2a_v2", trimmed);
    // WebSocket 客户端需要 ws:// 或 wss:// scheme, 不能是 http:// 或 https://
    url.replace("https://", "wss://")
        .replace("http://", "ws://")
}

fn trim_trailing_slash(input: &str) -> &str {
    input.trim_end_matches('/')
}

fn decode_hex_audio(value: &str) -> Result<Vec<u8>, BoxError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.len() % 2 != 0 {
        return Err("MiniMax audio payload is not valid hex".into());
    }

    let mut bytes = Vec::with_capacity(trimmed.len() / 2);
    for index in (0..trimmed.len()).step_by(2) {
        let byte = u8::from_str_radix(&trimmed[index..index + 2], 16)
            .map_err(|error| format!("failed to decode MiniMax audio payload: {}", error))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

struct SynthesizeResult {
    audio_bytes: Vec<u8>,
    chunk_count: usize,
    trace_id: Option<String>,
}
