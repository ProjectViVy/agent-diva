use crate::provider_auth::{
    BrowserLoginSession, PendingDeviceCodeLogin, PreparedBrowserLogin, ProviderLoginMode,
    ProviderOAuthBackend, ResolvedOAuthLogin,
};
use agent_diva_core::auth::{
    generate_pkce_state, parse_code_from_redirect, OAuthTokenManager, ProviderTokenSet,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const OPENAI_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OPENAI_OAUTH_AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const OPENAI_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OPENAI_OAUTH_DEVICE_CODE_URL: &str = "https://auth.openai.com/oauth/device/code";
const OPENAI_OAUTH_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const OPENAI_SCOPE: &str = "openid profile email offline_access";

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
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
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    verification_uri_complete: Option<String>,
    expires_in: u64,
    #[serde(default)]
    interval: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct OpenAiCodexOAuthBackend {
    client: Client,
    authorize_url: String,
    token_url: String,
    device_code_url: String,
}

pub type OpenAiCodexAuthHandler = OpenAiCodexOAuthBackend;
pub type OpenAiCodexBrowserSession = BrowserLoginSession;

impl OpenAiCodexOAuthBackend {
    pub fn new(
        authorize_url: impl Into<String>,
        token_url: impl Into<String>,
        device_code_url: impl Into<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            authorize_url: authorize_url.into(),
            token_url: token_url.into(),
            device_code_url: device_code_url.into(),
        }
    }

    fn build_authorize_url(&self, session: &BrowserLoginSession) -> String {
        let mut params = BTreeMap::new();
        params.insert("response_type", "code");
        params.insert("client_id", OPENAI_OAUTH_CLIENT_ID);
        params.insert("redirect_uri", OPENAI_OAUTH_REDIRECT_URI);
        params.insert("scope", OPENAI_SCOPE);
        params.insert("code_challenge", session.code_challenge.as_str());
        params.insert("code_challenge_method", "S256");
        params.insert("state", session.state.as_str());
        params.insert("codex_cli_simplified_flow", "true");
        params.insert("id_token_add_organizations", "true");

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

    async fn exchange_code_for_tokens(
        &self,
        code: &str,
        session: &BrowserLoginSession,
    ) -> Result<ProviderTokenSet> {
        let response = self
            .client
            .post(&self.token_url)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("client_id", OPENAI_OAUTH_CLIENT_ID),
                ("redirect_uri", OPENAI_OAUTH_REDIRECT_URI),
                ("code_verifier", session.code_verifier.as_str()),
            ])
            .send()
            .await
            .context("Failed to exchange OpenAI Codex authorization code")?;
        self.parse_token_response(response).await
    }

    async fn parse_token_response(&self, response: reqwest::Response) -> Result<ProviderTokenSet> {
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI Codex token exchange failed ({status}): {body}");
        }
        let parsed: TokenResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI Codex token response")?;
        Ok(ProviderTokenSet {
            access_token: parsed.access_token,
            refresh_token: parsed.refresh_token,
            id_token: parsed.id_token,
            expires_at: parsed
                .expires_in
                .map(|value| Utc::now() + chrono::Duration::seconds(value)),
            token_type: parsed.token_type,
            scope: parsed.scope,
        })
    }
}

impl Default for OpenAiCodexOAuthBackend {
    fn default() -> Self {
        Self::new(
            OPENAI_OAUTH_AUTHORIZE_URL,
            OPENAI_OAUTH_TOKEN_URL,
            OPENAI_OAUTH_DEVICE_CODE_URL,
        )
    }
}

#[async_trait]
impl OAuthTokenManager for OpenAiCodexOAuthBackend {
    async fn refresh_tokens(&self, refresh_token: &str) -> Result<ProviderTokenSet> {
        let response = self
            .client
            .post(&self.token_url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", OPENAI_OAUTH_CLIENT_ID),
            ])
            .send()
            .await
            .context("Failed to refresh OpenAI Codex OAuth token")?;
        let mut refreshed = self.parse_token_response(response).await?;
        if refreshed.refresh_token.is_none() {
            refreshed.refresh_token = Some(refresh_token.to_string());
        }
        Ok(refreshed)
    }

    fn extract_account_id(&self, access_token: &str) -> Option<String> {
        extract_account_id_from_jwt(access_token)
    }
}

#[async_trait]
impl ProviderOAuthBackend for OpenAiCodexOAuthBackend {
    fn provider_name(&self) -> &'static str {
        "openai-codex"
    }

    fn supports_mode(&self, _mode: &ProviderLoginMode) -> bool {
        true
    }

    fn prepare_browser_login(&self) -> Result<PreparedBrowserLogin> {
        let pkce = generate_pkce_state();
        let session = BrowserLoginSession {
            state: pkce.state,
            code_verifier: pkce.code_verifier,
            code_challenge: pkce.code_challenge,
        };
        let authorize_url = self.build_authorize_url(&session);
        Ok(PreparedBrowserLogin {
            session,
            authorize_url,
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
        let (mut stream, _) = tokio::time::timeout(timeout, listener.accept())
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
        let code = parse_code_from_redirect(path, Some(session.state.as_str()))?;
        let body = "<html><body><h2>agent-diva login complete</h2><p>You can close this tab.</p></body></html>";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .await
            .context("Failed to write OAuth callback response")?;
        Ok(code)
    }

    async fn complete_browser_login(
        &self,
        code_or_redirect: &str,
        session: &BrowserLoginSession,
    ) -> Result<ResolvedOAuthLogin> {
        let code = parse_code_from_redirect(code_or_redirect, Some(session.state.as_str()))?;
        let token_set = self.exchange_code_for_tokens(&code, session).await?;
        Ok(ResolvedOAuthLogin {
            provider: self.provider_name().to_string(),
            account_id: extract_account_id_from_jwt(&token_set.access_token),
            token_set,
        })
    }

    async fn start_device_code_login(&self) -> Result<PendingDeviceCodeLogin> {
        let response = self
            .client
            .post(&self.device_code_url)
            .form(&[
                ("client_id", OPENAI_OAUTH_CLIENT_ID),
                ("scope", OPENAI_SCOPE),
            ])
            .send()
            .await
            .context("Failed to start OpenAI Codex device-code flow")?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI Codex device-code start failed ({status}): {body}");
        }
        let device: DeviceCodeResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI Codex device-code response")?;
        Ok(PendingDeviceCodeLogin {
            device_code: device.device_code,
            user_code: device.user_code,
            verification_uri: device.verification_uri,
            verification_uri_complete: device.verification_uri_complete,
            expires_in: device.expires_in,
            interval: device.interval.unwrap_or(5).max(1),
            message: None,
        })
    }

    async fn poll_device_code_login(
        &self,
        pending: &PendingDeviceCodeLogin,
    ) -> Result<ResolvedOAuthLogin> {
        let started = Instant::now();
        loop {
            if started.elapsed() > Duration::from_secs(pending.expires_in) {
                anyhow::bail!("OpenAI Codex device-code flow timed out");
            }
            tokio::time::sleep(Duration::from_secs(pending.interval)).await;
            let response = self
                .client
                .post(&self.token_url)
                .form(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", pending.device_code.as_str()),
                    ("client_id", OPENAI_OAUTH_CLIENT_ID),
                ])
                .send()
                .await
                .context("Failed polling OpenAI Codex device-code token endpoint")?;
            if response.status().is_success() {
                let token_set = self.parse_token_response(response).await?;
                return Ok(ResolvedOAuthLogin {
                    provider: self.provider_name().to_string(),
                    account_id: extract_account_id_from_jwt(&token_set.access_token),
                    token_set,
                });
            }
        }
    }
}

pub fn extract_account_id_from_jwt(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()
        .or_else(|| {
            base64::engine::general_purpose::URL_SAFE
                .decode(payload)
                .ok()
        })?;
    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    for key in ["https://api.openai.com/auth", "org_id", "account_id", "sub"] {
        if let Some(value) = json.get(key).and_then(|value| value.as_str()) {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

    #[tokio::test]
    async fn exchange_code_success_path() {
        let mut server = Server::new_async().await;
        let token_mock = server
            .mock("POST", "/oauth/token")
            .match_body(Matcher::Regex("grant_type=authorization_code".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"access_token":"access","refresh_token":"refresh","expires_in":3600,"token_type":"Bearer","scope":"openid"}"#,
            )
            .create_async()
            .await;

        let handler = OpenAiCodexOAuthBackend::new(
            server.url(),
            format!("{}/oauth/token", server.url()),
            format!("{}/oauth/device/code", server.url()),
        );
        let prepared = handler.prepare_browser_login().unwrap();
        let tokens = handler
            .complete_browser_login("code", &prepared.session)
            .await
            .unwrap();
        token_mock.assert_async().await;
        assert_eq!(tokens.token_set.access_token, "access");
        assert_eq!(tokens.token_set.refresh_token.as_deref(), Some("refresh"));
    }
}
