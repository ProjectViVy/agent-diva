# P3-15: ProviderError::ApiError(String) 需结构化

## 问题描述

`agent-diva-providers/src/base.rs` 中 `ProviderError` 把上游 API 错误定义为纯字符串：

```rust
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}
```

`agent-diva-providers/src/litellm.rs` 在非 2xx 响应时将 status 与 body 拼成字符串：

```rust
return Err(ProviderError::ApiError(format!(
    "HTTP {}: {}",
    status, error_text
)));
```

目前 `provider_error_indicates_vision_unsupported` 和 `provider_error_indicates_context_overflow` 只能对 `ApiError(String)` 做字符串关键词匹配。这导致 status code、provider、model、error type、request id、retry-after 等结构化信息无法保留。

## 影响评估

- 可观测性影响：日志和 UI 只能显示拼接字符串，无法按 status、provider、model 聚合分析。
- 恢复策略受限：无法可靠区分 401/403 配置错误、429 rate limit、5xx 可重试错误、400 参数错误。
- 用户体验影响：前端或 CLI 难以给出精确提示，例如“API key 无效”“模型不支持图片”“上下文超限”。
- 兼容风险：所有 provider 都可能用不同文本拼接格式，调用方只能写脆弱的字符串解析逻辑。

## 解决方案

新增结构化 API error 类型，保留 Display 输出兼容人类可读错误。

示例：

```rust
#[derive(Debug, Clone)]
pub struct ProviderApiError {
    pub status: Option<u16>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub code: Option<String>,
    pub message: String,
    pub error_type: Option<String>,
    pub retry_after_secs: Option<u64>,
    pub request_id: Option<String>,
}

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("API error ({status:?}): {message}", status = .0.status, message = .0.message)]
    ApiError(ProviderApiError),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
```

LiteLLM non-success response 应解析常见 OpenAI-compatible 错误体：

```rust
#[derive(Deserialize)]
struct OpenAiErrorEnvelope {
    error: Option<OpenAiErrorBody>,
}

#[derive(Deserialize)]
struct OpenAiErrorBody {
    message: Option<String>,
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: Option<serde_json::Value>,
}
```

构造错误：

```rust
return Err(ProviderError::ApiError(ProviderApiError {
    status: Some(status.as_u16()),
    provider: self.selected_provider.as_ref().map(|p| p.name.clone()),
    model: Some(resolved_model.clone()),
    code,
    message,
    error_type,
    retry_after_secs,
    request_id,
}));
```

迁移建议：

- 先添加 `ProviderApiError` 和 helper：`ProviderApiError::message()`.
- 更新 `provider_error_indicates_*` 同时读取结构字段和 message。
- 为旧调用点提供构造器：`ProviderError::api_message(message)`，降低迁移成本。
- 更新 tests 中 `ProviderError::ApiError("...".to_string())`。

## 验证方法

执行：

```powershell
cargo test -p agent-diva-providers provider_error
cargo test -p agent-diva-providers litellm
just fmt-check
just check
```

应新增测试：

- 400/401/429/500 响应能保留 `status`。
- OpenAI-compatible `{ "error": { "message", "type", "code" } }` 能解析到结构字段。
- 非 JSON error body 仍能落入 `message`。
- vision unsupported 与 context overflow 检测在结构化错误后仍通过。
- Display 文本仍对用户可读，不泄露 API key 或完整请求体。

## 优先级

P3
