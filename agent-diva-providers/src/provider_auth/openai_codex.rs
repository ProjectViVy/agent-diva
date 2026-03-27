use super::{ProviderLoginMode, ProviderLoginRequest, ProviderLoginResult};
use agent_diva_core::auth::{extract_account_id_from_jwt, ProviderAuthService, ProviderTokenSet};
use anyhow::{Context, Result};
use base64::Engine;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
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

#[derive(Debug, Clone)]
pub struct OpenAiCodexBrowserSession {
    state: String,
    code_verifier: String,
    code_challenge: String,
    authorize_url: String,
}

impl OpenAiCodexBrowserSession {
    pub fn authorize_url(&self) -> &str {
        &self.authorize_url
    }
}

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
pub struct OpenAiCodexAuthHandler {
    client: Client,
    authorize_url: String,
    token_url: String,
    device_code_url: String,
}

impl OpenAiCodexAuthHandler {
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

    pub async fn login(
        &self,
        auth_service: &ProviderAuthService,
        request: ProviderLoginRequest,
    ) -> Result<ProviderLoginResult> {
        let token_set = match request.mode {
            ProviderLoginMode::Browser => {
                let session = self.prepare_browser_login();
                let authorize_url = session.authorize_url().to_string();
                println!(
                    "Open this URL in your browser to continue:\n\n{}\n",
                    authorize_url
                );
                let code = receive_loopback_code(session.state.as_str(), Duration::from_secs(180))
                    .await
                    .or_else(|_| {
                        anyhow::bail!(
                            "Browser callback timed out. Retry with `agent-diva provider login openai-codex --paste-code`."
                        )
                    })?;
                self.exchange_code_for_tokens(&code, &session).await?
            }
            ProviderLoginMode::PasteRedirect { input } => {
                let session = self.prepare_browser_login();
                let authorize_url = session.authorize_url().to_string();
                if let Some(input) = input {
                    let code = parse_code_from_redirect(&input, session.state.as_str())?;
                    self.exchange_code_for_tokens(&code, &session).await?
                } else {
                    println!(
                        "Open this URL in your browser and paste the final redirect URL:\n\n{}\n",
                        authorize_url
                    );
                    anyhow::bail!("Missing redirect input for paste flow")
                }
            }
            ProviderLoginMode::DeviceCode => self.device_code_login().await?,
        };
        let profile = auth_service
            .store_openai_codex_tokens(&request.profile_name, token_set.clone())
            .await?;
        Ok(ProviderLoginResult {
            provider: "openai-codex".to_string(),
            profile_name: request.profile_name,
            account_id: profile
                .account_id
                .or_else(|| extract_account_id_from_jwt(&token_set.access_token)),
            status: "authenticated".to_string(),
        })
    }

    pub fn prepare_browser_login(&self) -> OpenAiCodexBrowserSession {
        let mut session = generate_pkce_state();
        session.authorize_url = self.build_authorize_url(&session);
        session
    }

    pub async fn wait_for_browser_callback(
        &self,
        session: &OpenAiCodexBrowserSession,
        timeout: Duration,
    ) -> Result<String> {
        receive_loopback_code(session.state.as_str(), timeout).await
    }

    pub async fn complete_browser_login(
        &self,
        auth_service: &ProviderAuthService,
        profile_name: &str,
        session: &OpenAiCodexBrowserSession,
        redirect_input: &str,
    ) -> Result<ProviderLoginResult> {
        let code = parse_code_from_redirect(redirect_input, session.state.as_str())?;
        let token_set = self.exchange_code_for_tokens(&code, session).await?;
        self.store_login_result(auth_service, profile_name, token_set)
            .await
    }

    pub async fn complete_browser_login_with_code(
        &self,
        auth_service: &ProviderAuthService,
        profile_name: &str,
        session: &OpenAiCodexBrowserSession,
        code: &str,
    ) -> Result<ProviderLoginResult> {
        let token_set = self.exchange_code_for_tokens(code, session).await?;
        self.store_login_result(auth_service, profile_name, token_set)
            .await
    }

    fn build_authorize_url(&self, pkce: &OpenAiCodexBrowserSession) -> String {
        let mut params = BTreeMap::new();
        params.insert("response_type", "code");
        params.insert("client_id", OPENAI_OAUTH_CLIENT_ID);
        params.insert("redirect_uri", OPENAI_OAUTH_REDIRECT_URI);
        params.insert("scope", OPENAI_SCOPE);
        params.insert("code_challenge", pkce.code_challenge.as_str());
        params.insert("code_challenge_method", "S256");
        params.insert("state", pkce.state.as_str());
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
        pkce: &OpenAiCodexBrowserSession,
    ) -> Result<ProviderTokenSet> {
        let response = self
            .client
            .post(&self.token_url)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("client_id", OPENAI_OAUTH_CLIENT_ID),
                ("redirect_uri", OPENAI_OAUTH_REDIRECT_URI),
                ("code_verifier", pkce.code_verifier.as_str()),
            ])
            .send()
            .await
            .context("Failed to exchange OpenAI Codex authorization code")?;
        self.parse_token_response(response).await
    }

    async fn device_code_login(&self) -> Result<ProviderTokenSet> {
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
        println!(
            "Open {} and enter code {}",
            device
                .verification_uri_complete
                .clone()
                .unwrap_or(device.verification_uri.clone()),
            device.user_code
        );

        let started = Instant::now();
        let interval = device.interval.unwrap_or(5).max(1);
        loop {
            if started.elapsed() > Duration::from_secs(device.expires_in) {
                anyhow::bail!("OpenAI Codex device-code flow timed out");
            }
            tokio::time::sleep(Duration::from_secs(interval)).await;
            let response = self
                .client
                .post(&self.token_url)
                .form(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", device.device_code.as_str()),
                    ("client_id", OPENAI_OAUTH_CLIENT_ID),
                ])
                .send()
                .await
                .context("Failed polling OpenAI Codex device-code token endpoint")?;
            if response.status().is_success() {
                return self.parse_token_response(response).await;
            }
        }
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

    async fn store_login_result(
        &self,
        auth_service: &ProviderAuthService,
        profile_name: &str,
        token_set: ProviderTokenSet,
    ) -> Result<ProviderLoginResult> {
        let profile = auth_service
            .store_openai_codex_tokens(profile_name, token_set.clone())
            .await?;
        Ok(ProviderLoginResult {
            provider: "openai-codex".to_string(),
            profile_name: profile_name.to_string(),
            account_id: profile
                .account_id
                .or_else(|| extract_account_id_from_jwt(&token_set.access_token)),
            status: "authenticated".to_string(),
        })
    }
}

impl Default for OpenAiCodexAuthHandler {
    fn default() -> Self {
        Self::new(
            OPENAI_OAUTH_AUTHORIZE_URL,
            OPENAI_OAUTH_TOKEN_URL,
            OPENAI_OAUTH_DEVICE_CODE_URL,
        )
    }
}

fn generate_pkce_state() -> OpenAiCodexBrowserSession {
    let state = uuid::Uuid::new_v4().to_string();
    let verifier = format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    OpenAiCodexBrowserSession {
        state,
        code_verifier: verifier,
        code_challenge: challenge,
        authorize_url: String::new(),
    }
}

async fn receive_loopback_code(expected_state: &str, timeout: Duration) -> Result<String> {
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
    let code = parse_code_from_redirect(path, expected_state)?;
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

fn parse_code_from_redirect(input: &str, expected_state: &str) -> Result<String> {
    let query = input
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or(input);
    let params = query
        .split('&')
        .filter_map(|entry| entry.split_once('='))
        .map(|(key, value)| {
            (
                urlencoding::decode(key)
                    .unwrap_or_else(|_| key.into())
                    .to_string(),
                urlencoding::decode(value)
                    .unwrap_or_else(|_| value.into())
                    .to_string(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    if let Some(state) = params.get("state") {
        if state != expected_state {
            anyhow::bail!("OAuth state mismatch");
        }
    }
    params
        .get("code")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("OAuth redirect does not contain authorization code"))
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

        let handler = OpenAiCodexAuthHandler::new(
            server.url(),
            format!("{}/oauth/token", server.url()),
            format!("{}/oauth/device/code", server.url()),
        );
        let pkce = generate_pkce_state();
        let tokens = handler
            .exchange_code_for_tokens("code", &pkce)
            .await
            .unwrap();
        token_mock.assert_async().await;
        assert_eq!(tokens.access_token, "access");
        assert_eq!(tokens.refresh_token.as_deref(), Some("refresh"));
    }
}
