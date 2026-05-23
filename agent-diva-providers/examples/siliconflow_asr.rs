use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

type BoxError = Box<dyn Error + Send + Sync>;

const DEFAULT_BASE_URL: &str = "https://api.siliconflow.cn/v1";
const DEFAULT_MODEL: &str = "FunAudioLLM/SenseVoiceSmall";

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let api_key = required_env("SILICONFLOW_API_KEY")?;
    let audio_path = resolve_audio_path()?;
    let base_url =
        env::var("SILICONFLOW_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let model = env::var("SILICONFLOW_ASR_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let language = env::var("SILICONFLOW_ASR_LANGUAGE").ok();

    println!("SiliconFlow ASR demo started");
    println!("base_url     : {}", base_url);
    println!("model        : {}", model);
    println!(
        "language     : {}",
        language.clone().unwrap_or_else(|| "auto".to_string())
    );
    println!("audio_path   : {}", audio_path.display());

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let result =
        transcribe_audio(&client, &api_key, &base_url, &model, language, &audio_path).await?;

    println!(
        "trace_id     : {}",
        result.trace_id.unwrap_or_else(|| "n/a".to_string())
    );
    println!("transcription: {}", result.text);
    println!("done");
    Ok(())
}

fn required_env(key: &str) -> Result<String, BoxError> {
    env::var(key).map_err(|_| format!("missing required environment variable: {}", key).into())
}

fn resolve_audio_path() -> Result<PathBuf, BoxError> {
    if let Some(arg_path) = env::args().nth(1) {
        if !arg_path.trim().is_empty() {
            return Ok(PathBuf::from(arg_path));
        }
    }

    if let Ok(env_path) = env::var("SILICONFLOW_ASR_AUDIO_PATH") {
        if !env_path.trim().is_empty() {
            return Ok(PathBuf::from(env_path));
        }
    }

    Err(
        "missing audio path: pass it as the first CLI argument or set SILICONFLOW_ASR_AUDIO_PATH"
            .into(),
    )
}

async fn transcribe_audio(
    client: &Client,
    api_key: &str,
    base_url: &str,
    model: &str,
    language: Option<String>,
    audio_path: &PathBuf,
) -> Result<TranscriptionResult, BoxError> {
    let file_bytes = fs::read(audio_path).await?;
    let file_name = audio_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("audio.webm")
        .to_string();

    let audio_part = Part::bytes(file_bytes).file_name(file_name);
    let mut form = Form::new()
        .part("file", audio_part)
        .text("model", model.to_string());

    if let Some(language) = language.filter(|value| !value.trim().is_empty()) {
        form = form.text("language", language);
    }

    let response = client
        .post(format!(
            "{}/audio/transcriptions",
            trim_trailing_slash(base_url)
        ))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    let headers = response.headers().clone();
    let body_text = response.text().await?;
    if !status.is_success() {
        return Err(format!(
            "SiliconFlow ASR request failed: status={} body={}",
            status, body_text
        )
        .into());
    }

    let json = serde_json::from_str::<Value>(&body_text)
        .map_err(|error| format!("invalid JSON response: {} | body={}", error, body_text))?;
    let text = json
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("ASR response did not contain text: {}", body_text))?
        .to_string();

    Ok(TranscriptionResult {
        text,
        trace_id: headers
            .get("x-siliconcloud-trace-id")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned),
    })
}

fn trim_trailing_slash(input: &str) -> &str {
    input.trim_end_matches('/')
}

struct TranscriptionResult {
    text: String,
    trace_id: Option<String>,
}
