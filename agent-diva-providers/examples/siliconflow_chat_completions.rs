use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::time::Duration;

type BoxError = Box<dyn Error + Send + Sync>;

const DEFAULT_BASE_URL: &str = "https://api.siliconflow.cn/v1";
const DEFAULT_MODEL: &str = "Qwen/Qwen3-8B";

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let api_key = required_env("SILICONFLOW_API_KEY")?;
    let prompt = resolve_prompt()?;
    let base_url =
        env::var("SILICONFLOW_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let model = env::var("SILICONFLOW_CHAT_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let max_tokens = env::var("SILICONFLOW_CHAT_MAX_TOKENS")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(256);
    let temperature = env::var("SILICONFLOW_CHAT_TEMPERATURE")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.7);

    println!("SiliconFlow chat/completions demo started");
    println!("base_url     : {}", base_url);
    println!("model        : {}", model);
    println!("max_tokens   : {}", max_tokens);
    println!("temperature  : {}", temperature);
    println!("prompt_len   : {}", prompt.chars().count());

    let client = Client::builder().timeout(Duration::from_secs(90)).build()?;
    let result = chat_once(
        &client,
        &api_key,
        &base_url,
        &model,
        &prompt,
        max_tokens,
        temperature,
    )
    .await?;

    println!(
        "trace_id     : {}",
        result.trace_id.unwrap_or_else(|| "n/a".to_string())
    );
    println!("finish_reason: {}", result.finish_reason);
    if let Some(reasoning) = result.reasoning_content {
        println!("reasoning    : {}", reasoning);
    }
    println!("assistant    : {}", result.assistant_content);
    println!("done");
    Ok(())
}

fn required_env(key: &str) -> Result<String, BoxError> {
    env::var(key).map_err(|_| format!("missing required environment variable: {}", key).into())
}

fn resolve_prompt() -> Result<String, BoxError> {
    let cli_prompt = env::args().nth(1).unwrap_or_default();
    if !cli_prompt.trim().is_empty() {
        return Ok(cli_prompt);
    }

    let env_prompt = env::var("SILICONFLOW_CHAT_PROMPT").unwrap_or_default();
    if !env_prompt.trim().is_empty() {
        return Ok(env_prompt);
    }

    Err("missing prompt: pass it as the first CLI argument or set SILICONFLOW_CHAT_PROMPT".into())
}

async fn chat_once(
    client: &Client,
    api_key: &str,
    base_url: &str,
    model: &str,
    prompt: &str,
    max_tokens: i32,
    temperature: f64,
) -> Result<ChatResult, BoxError> {
    let url = format!("{}/chat/completions", trim_trailing_slash(base_url));
    let payload = json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "stream": false,
        "max_tokens": max_tokens,
        "temperature": temperature
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
    let body_text = response.text().await?;
    if !status.is_success() {
        return Err(format!(
            "SiliconFlow chat/completions request failed: status={} body={}",
            status, body_text
        )
        .into());
    }

    let json = serde_json::from_str::<Value>(&body_text)
        .map_err(|error| format!("invalid JSON response: {} | body={}", error, body_text))?;
    let choice = json
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| format!("chat/completions response missing choices: {}", body_text))?;

    let message = choice
        .get("message")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("chat/completions response missing message: {}", body_text))?;

    let assistant_content = message
        .get("content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "chat/completions response missing assistant content: {}",
                body_text
            )
        })?
        .to_string();

    let finish_reason = choice
        .get("finish_reason")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();

    let reasoning_content = message
        .get("reasoning_content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    Ok(ChatResult {
        assistant_content,
        finish_reason,
        reasoning_content,
        trace_id: headers
            .get("x-siliconcloud-trace-id")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned),
    })
}

fn trim_trailing_slash(input: &str) -> &str {
    input.trim_end_matches('/')
}

struct ChatResult {
    assistant_content: String,
    finish_reason: String,
    reasoning_content: Option<String>,
    trace_id: Option<String>,
}
