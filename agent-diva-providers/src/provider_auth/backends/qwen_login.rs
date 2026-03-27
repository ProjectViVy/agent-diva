use crate::provider_auth::{
    BrowserLoginSession, PendingDeviceCodeLogin, PreparedBrowserLogin, ProviderLoginMode,
    ProviderOAuthBackend, ResolvedOAuthLogin,
};
use agent_diva_core::auth::{
    generate_pkce_state, parse_code_from_redirect, OAuthProfileState, OAuthTokenManager,
    ProviderTokenSet,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const QWEN_OAUTH_CLIENT_ID: &str = "f0304373b74a44d2b584a3fb70ca9e56";
const QWEN_OAUTH_AUTHORIZE_URL: &str = "https://chat.qwen.ai/api/v1/oauth2/authorize";
const QWEN_OAUTH_TOKEN_URL: &str = "https://chat.qwen.ai/api/v1/oauth2/token";
const QWEN_OAUTH_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const QWEN_OAUTH_SCOPE: &str = "openid profile offline_access";
const QWEN_DEFAULT_API_BASE: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

#[derive(Debug, Deserialize)]
struct QwenTokenResponse {
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    resource_url: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QwenLoginOAuthBackend {
    client: Client,
    authorize_url: String,
    token_url: String,
}

impl QwenLoginOAuthBackend {
    pub fn new(authorize_url: impl Into<String>, token_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            authorize_url: authorize_url.into(),
            token_url: token_url.into(),
        }
    }

    fn build_authorize_url(&self, session: &BrowserLoginSession) -> String {
        let params = [
            ("response_type", "code"),
            ("client_id", QWEN_OAUTH_CLIENT_ID),
            ("redirect_uri", QWEN_OAUTH_REDIRECT_URI),
            ("scope", QWEN_OAUTH_SCOPE),
            ("code_challenge", session.code_challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("state", session.state.as_str()),
        ];
        let query = params
            .into_iter()
            .map(|(key, value)| {
                format!(
                    "{}={}",
                    urlencoding::encode(key),
                    urlencoding::encode(value)
                )
            })
            .collect::<Vec<_>>()
            .join("&");
        format!("{}?{}", self.authorize_url, query)
    }

    fn normalize_api_base(resource_url: Option<&str>) -> String {
        let Some(raw) = resource_url
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return QWEN_DEFAULT_API_BASE.to_string();
        };
        let with_scheme = if raw.starts_with("http://") || raw.starts_with("https://") {
            raw.to_string()
        } else {
            format!("https://{raw}")
        };
        let normalized = with_scheme.trim_end_matches('/').to_string();
        if normalized.ends_with("/v1") {
            normalized
        } else {
            format!("{normalized}/v1")
        }
    }

    fn metadata_from_response(parsed: &QwenTokenResponse) -> BTreeMap<String, String> {
        let api_base = Self::normalize_api_base(parsed.resource_url.as_deref());
        let mut metadata = BTreeMap::from([("api_base".to_string(), api_base)]);
        if let Some(resource_url) = parsed
            .resource_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            metadata.insert("resource_url".to_string(), resource_url.to_string());
        }
        metadata
    }

    async fn exchange_code_for_state(
        &self,
        code: &str,
        session: &BrowserLoginSession,
    ) -> Result<OAuthProfileState> {
        let response = self
            .client
            .post(&self.token_url)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("client_id", QWEN_OAUTH_CLIENT_ID),
                ("redirect_uri", QWEN_OAUTH_REDIRECT_URI),
                ("code_verifier", session.code_verifier.as_str()),
            ])
            .send()
            .await
            .context("Failed to exchange Qwen Login authorization code")?;
        self.parse_profile_state(response).await
    }

    async fn parse_profile_state(&self, response: reqwest::Response) -> Result<OAuthProfileState> {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let parsed: QwenTokenResponse =
            serde_json::from_str(&body).context("Failed to parse Qwen Login token response")?;

        if !status.is_success() {
            let detail = parsed
                .error_description
                .as_deref()
                .or(parsed.error.as_deref())
                .unwrap_or(body.as_str());
            anyhow::bail!("Qwen Login token exchange failed ({status}): {detail}");
        }

        if let Some(error) = parsed
            .error
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let detail = parsed.error_description.as_deref().unwrap_or(error);
            anyhow::bail!("Qwen Login token exchange failed: {detail}");
        }

        let access_token = parsed
            .access_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Qwen Login token response missing access_token"))?
            .to_string();

        Ok(OAuthProfileState {
            token_set: ProviderTokenSet {
                access_token,
                refresh_token: parsed
                    .refresh_token
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string),
                id_token: parsed
                    .id_token
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string),
                expires_at: parsed
                    .expires_in
                    .map(|value| Utc::now() + chrono::Duration::seconds(value)),
                token_type: parsed
                    .token_type
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string),
                scope: parsed
                    .scope
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string),
            },
            account_id: None,
            metadata: Self::metadata_from_response(&parsed),
        })
    }
}

impl Default for QwenLoginOAuthBackend {
    fn default() -> Self {
        Self::new(QWEN_OAUTH_AUTHORIZE_URL, QWEN_OAUTH_TOKEN_URL)
    }
}

#[async_trait]
impl OAuthTokenManager for QwenLoginOAuthBackend {
    async fn refresh_oauth_state(&self, refresh_token: &str) -> Result<OAuthProfileState> {
        let response = self
            .client
            .post(&self.token_url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", QWEN_OAUTH_CLIENT_ID),
            ])
            .send()
            .await
            .context("Failed to refresh Qwen Login OAuth token")?;
        let mut refreshed = self.parse_profile_state(response).await?;
        if refreshed.token_set.refresh_token.is_none() {
            refreshed.token_set.refresh_token = Some(refresh_token.to_string());
        }
        Ok(refreshed)
    }

    fn extract_account_id(&self, _access_token: &str) -> Option<String> {
        None
    }
}

#[async_trait]
impl ProviderOAuthBackend for QwenLoginOAuthBackend {
    fn provider_name(&self) -> &'static str {
        "qwen-login"
    }

    fn supports_mode(&self, mode: &ProviderLoginMode) -> bool {
        matches!(
            mode,
            ProviderLoginMode::Browser | ProviderLoginMode::PasteRedirect { .. }
        )
    }

    fn prepare_browser_login(&self) -> Result<PreparedBrowserLogin> {
        let pkce = generate_pkce_state();
        let session = BrowserLoginSession {
            state: pkce.state,
            code_verifier: pkce.code_verifier,
            code_challenge: pkce.code_challenge,
        };
        Ok(PreparedBrowserLogin {
            authorize_url: self.build_authorize_url(&session),
            session,
        })
    }

    async fn wait_for_browser_callback(
        &self,
        session: &BrowserLoginSession,
        timeout: Duration,
    ) -> Result<String> {
        let listener = TcpListener::bind("127.0.0.1:1455")
            .await
            .context("Failed to bind loopback callback listener")?;
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                anyhow::bail!("Timed out waiting for OAuth callback");
            }

            let (mut stream, _) = tokio::time::timeout(remaining, listener.accept())
                .await
                .context("Timed out waiting for OAuth callback")?
                .context("Failed to accept OAuth callback connection")?;

            let mut buffer = vec![0_u8; 8192];
            let bytes_read = stream
                .read(&mut buffer)
                .await
                .context("Failed to read OAuth callback request")?;
            let request = String::from_utf8_lossy(&buffer[..bytes_read]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .ok_or_else(|| anyhow::anyhow!("Malformed OAuth callback request"))?;

            let body = match parse_code_from_redirect(path, Some(&session.state)) {
                Ok(code) => {
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nQwen Login authentication completed. You can return to Agent Diva.",
                        )
                        .await
                        .context("Failed to write OAuth callback response")?;
                    return Ok(code);
                }
                Err(error) => {
                    let message = format!(
                        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\n\r\n{}\n",
                        error
                    );
                    message
                }
            };

            stream
                .write_all(body.as_bytes())
                .await
                .context("Failed to write OAuth callback response")?;
        }
    }

    async fn complete_browser_login(
        &self,
        code_or_redirect: &str,
        session: &BrowserLoginSession,
    ) -> Result<ResolvedOAuthLogin> {
        let code = parse_code_from_redirect(code_or_redirect, Some(&session.state))
            .unwrap_or_else(|_| code_or_redirect.trim().to_string());
        let state = self.exchange_code_for_state(&code, session).await?;
        Ok(ResolvedOAuthLogin {
            provider: "qwen-login".to_string(),
            token_set: state.token_set,
            account_id: state.account_id,
            metadata: state.metadata,
        })
    }

    async fn start_device_code_login(&self) -> Result<PendingDeviceCodeLogin> {
        anyhow::bail!("Provider 'qwen-login' does not support login mode 'DeviceCode'")
    }

    async fn poll_device_code_login(
        &self,
        _pending: &PendingDeviceCodeLogin,
    ) -> Result<ResolvedOAuthLogin> {
        anyhow::bail!("Provider 'qwen-login' does not support login mode 'DeviceCode'")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;

    #[tokio::test]
    async fn exchange_code_success_path() {
        let mut server = mockito::Server::new_async().await;
        let backend = QwenLoginOAuthBackend::new("https://example.com/authorize", server.url());
        let session = BrowserLoginSession {
            state: "state".to_string(),
            code_verifier: "verifier".to_string(),
            code_challenge: "challenge".to_string(),
        };
        let _mock = server
            .mock("POST", "/")
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded("grant_type".into(), "authorization_code".into()),
                Matcher::UrlEncoded("code".into(), "code-123".into()),
                Matcher::UrlEncoded("client_id".into(), QWEN_OAUTH_CLIENT_ID.into()),
            ]))
            .with_status(200)
            .with_body(
                r#"{
                    "access_token":"qwen-access",
                    "refresh_token":"qwen-refresh",
                    "expires_in":3600,
                    "resource_url":"https://dashscope.aliyuncs.com/compatible-mode"
                }"#,
            )
            .create_async()
            .await;

        let resolved = backend
            .complete_browser_login("code-123", &session)
            .await
            .unwrap();
        assert_eq!(resolved.provider, "qwen-login");
        assert_eq!(resolved.token_set.access_token, "qwen-access");
        assert_eq!(
            resolved.metadata.get("api_base").map(String::as_str),
            Some("https://dashscope.aliyuncs.com/compatible-mode/v1")
        );
    }

    #[tokio::test]
    async fn refresh_oauth_state_preserves_api_base_and_refresh_token() {
        let mut server = mockito::Server::new_async().await;
        let backend = QwenLoginOAuthBackend::new("https://example.com/authorize", server.url());
        let _mock = server
            .mock("POST", "/")
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded("grant_type".into(), "refresh_token".into()),
                Matcher::UrlEncoded("refresh_token".into(), "refresh-123".into()),
                Matcher::UrlEncoded("client_id".into(), QWEN_OAUTH_CLIENT_ID.into()),
            ]))
            .with_status(200)
            .with_body(
                r#"{
                    "access_token":"qwen-access-2",
                    "expires_in":3600,
                    "resource_url":"oauth.example.com/compatible-mode"
                }"#,
            )
            .create_async()
            .await;

        let refreshed = backend.refresh_oauth_state("refresh-123").await.unwrap();
        assert_eq!(refreshed.token_set.access_token, "qwen-access-2");
        assert_eq!(
            refreshed.token_set.refresh_token.as_deref(),
            Some("refresh-123")
        );
        assert_eq!(
            refreshed.metadata.get("api_base").map(String::as_str),
            Some("https://oauth.example.com/compatible-mode/v1")
        );
    }
}
