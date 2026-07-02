use reqwest::Client;
use serde_json::json;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

type BoxError = Box<dyn Error + Send + Sync>;

const DEFAULT_BASE_URL: &str = "https://api.siliconflow.cn/v1";
const DEFAULT_MODEL: &str = "fnlp/MOSS-TTSD-v0.5";
const DEFAULT_VOICE: &str = "fnlp/MOSS-TTSD-v0.5:anna";
const DEFAULT_FORMAT: &str = "mp3";
const DEFAULT_OUTPUT_STEM: &str = "siliconflow_tts_output";

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let api_key = required_env("SILICONFLOW_API_KEY")?;
    let text = resolve_text()?;
    let base_url =
        env::var("SILICONFLOW_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let model = env::var("SILICONFLOW_TTS_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let voice = env::var("SILICONFLOW_TTS_VOICE").unwrap_or_else(|_| DEFAULT_VOICE.to_string());
    let response_format =
        env::var("SILICONFLOW_TTS_FORMAT").unwrap_or_else(|_| DEFAULT_FORMAT.to_string());
    let speed = env::var("SILICONFLOW_TTS_SPEED")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(1.0);
    let gain = env::var("SILICONFLOW_TTS_GAIN")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    let output_path = resolve_output_path("SILICONFLOW_TTS_OUTPUT_PATH", &response_format);

    println!("SiliconFlow TTS demo started");
    println!("base_url     : {}", base_url);
    println!("model        : {}", model);
    println!("voice        : {}", voice);
    println!("format       : {}", response_format);
    println!("output_path  : {}", output_path.display());
    println!("text_length  : {}", text.chars().count());

    let client = Client::builder().timeout(Duration::from_secs(60)).build()?;
    let result = synthesize_speech(
        &client,
        &api_key,
        &base_url,
        &model,
        &voice,
        &response_format,
        speed,
        gain,
        &text,
    )
    .await?;

    fs::write(&output_path, &result.audio_bytes).await?;
    println!(
        "trace_id     : {}",
        result.trace_id.unwrap_or_else(|| "n/a".to_string())
    );
    println!("content_type : {}", result.content_type);
    println!("audio_bytes  : {}", result.audio_bytes.len());
    println!("saved_audio  : {}", output_path.display());
    println!("done");
    Ok(())
}

fn resolve_text() -> Result<String, BoxError> {
    let cli_text = env::args().nth(1).unwrap_or_default();
    if !cli_text.trim().is_empty() {
        return Ok(cli_text);
    }

    let env_text = env::var("SILICONFLOW_TTS_TEXT").unwrap_or_default();
    if !env_text.trim().is_empty() {
        return Ok(env_text);
    }

    Err("missing text: pass it as the first CLI argument or set SILICONFLOW_TTS_TEXT".into())
}

fn required_env(key: &str) -> Result<String, BoxError> {
    env::var(key).map_err(|_| format!("missing required environment variable: {}", key).into())
}

fn resolve_output_path(env_key: &str, format: &str) -> PathBuf {
    match env::var(env_key) {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => PathBuf::from(format!("{}.{}", DEFAULT_OUTPUT_STEM, format)),
    }
}

async fn synthesize_speech(
    client: &Client,
    api_key: &str,
    base_url: &str,
    model: &str,
    voice: &str,
    response_format: &str,
    speed: f64,
    gain: f64,
    text: &str,
) -> Result<SynthesizeResult, BoxError> {
    let url = format!("{}/audio/speech", trim_trailing_slash(base_url));
    let payload = json!({
        "model": model,
        "input": text,
        "voice": voice,
        "response_format": response_format,
        "speed": speed,
        "gain": gain,
        "stream": false
    });

    let response = client
        .post(url)
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.bytes().await?;
    if !status.is_success() {
        let body_preview = String::from_utf8_lossy(&bytes);
        return Err(format!(
            "SiliconFlow TTS request failed: status={} body={}",
            status, body_preview
        )
        .into());
    }

    Ok(SynthesizeResult {
        audio_bytes: bytes.to_vec(),
        content_type: headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string(),
        trace_id: headers
            .get("x-siliconcloud-trace-id")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned),
    })
}

fn trim_trailing_slash(input: &str) -> &str {
    input.trim_end_matches('/')
}

struct SynthesizeResult {
    audio_bytes: Vec<u8>,
    content_type: String,
    trace_id: Option<String>,
}
