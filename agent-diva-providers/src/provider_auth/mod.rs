mod backends;

use crate::registry::{AuthMode, ProviderRegistry};
use agent_diva_core::auth::{
    OAuthProfileState, OAuthTokenManager, ProviderAuthProfile, ProviderAuthService,
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

pub use backends::{
    extract_account_id_from_jwt, OpenAiCodexAuthHandler, OpenAiCodexBrowserSession,
    OpenAiCodexOAuthBackend, QwenLoginOAuthBackend,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrowserLoginSession {
    pub state: String,
    pub code_verifier: String,
    pub code_challenge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreparedBrowserLogin {
    pub session: BrowserLoginSession,
    pub authorize_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingDeviceCodeLogin {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in_seconds: u64,
    pub interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedOAuthLogin {
    pub provider: String,
    pub token_set: agent_diva_core::auth::ProviderTokenSet,
    pub account_id: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderLoginRequest {
    pub provider: String,
    pub profile_name: String,
    pub mode: ProviderLoginMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderLoginMode {
    Browser,
    DeviceCode,
    PasteRedirect { input: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderLoginResult {
    pub provider: String,
    pub profile_name: String,
    pub account_id: Option<String>,
    pub status: String,
}

#[async_trait]
pub trait ProviderOAuthBackend: OAuthTokenManager + Send + Sync {
    fn provider_name(&self) -> &'static str;

    fn supports_mode(&self, mode: &ProviderLoginMode) -> bool;

    fn prepare_browser_login(&self) -> Result<PreparedBrowserLogin>;

    async fn wait_for_browser_callback(
        &self,
        session: &BrowserLoginSession,
        timeout: Duration,
    ) -> Result<String>;

    async fn complete_browser_login(
        &self,
        code_or_redirect: &str,
        session: &BrowserLoginSession,
    ) -> Result<ResolvedOAuthLogin>;

    async fn start_device_code_login(&self) -> Result<PendingDeviceCodeLogin>;

    async fn poll_device_code_login(
        &self,
        pending: &PendingDeviceCodeLogin,
    ) -> Result<ResolvedOAuthLogin>;
}

#[derive(Clone, Default)]
pub struct ProviderLoginService {
    registry: ProviderRegistry,
    openai_codex: OpenAiCodexOAuthBackend,
    qwen_login: QwenLoginOAuthBackend,
}

impl ProviderLoginService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn supports_login(&self, provider: &str) -> bool {
        self.registry
            .find_by_name(provider)
            .is_some_and(|spec| spec.login_supported && spec.auth_mode == AuthMode::OAuth)
            && self.backend(provider).is_ok()
    }

    pub fn supports_mode(&self, provider: &str, mode: &ProviderLoginMode) -> bool {
        self.backend(provider)
            .is_ok_and(|backend| backend.supports_mode(mode))
    }

    pub fn supports_refresh(&self, provider: &str) -> bool {
        self.backend(provider).is_ok()
    }

    pub fn prepare_browser_login(&self, provider: &str) -> Result<PreparedBrowserLogin> {
        self.backend(provider)?.prepare_browser_login()
    }

    pub async fn wait_for_browser_callback(
        &self,
        provider: &str,
        session: &BrowserLoginSession,
        timeout: Duration,
    ) -> Result<String> {
        self.backend(provider)?
            .wait_for_browser_callback(session, timeout)
            .await
    }

    pub async fn complete_browser_login(
        &self,
        auth_service: &ProviderAuthService,
        provider: &str,
        profile_name: &str,
        session: &BrowserLoginSession,
        code_or_redirect: &str,
    ) -> Result<ProviderLoginResult> {
        let resolved = self
            .backend(provider)?
            .complete_browser_login(code_or_redirect, session)
            .await?;
        self.persist_resolved_login(auth_service, profile_name, resolved)
            .await
    }

    pub async fn refresh(
        &self,
        auth_service: &ProviderAuthService,
        provider: &str,
        profile_override: Option<&str>,
    ) -> Result<ProviderAuthProfile> {
        auth_service
            .refresh_oauth_profile(provider, profile_override, self.backend(provider)?)
            .await
    }

    pub async fn login(
        &self,
        auth_service: &ProviderAuthService,
        request: ProviderLoginRequest,
    ) -> Result<ProviderLoginResult> {
        let backend = self.backend(&request.provider)?;
        if !backend.supports_mode(&request.mode) {
            anyhow::bail!(
                "Provider '{}' does not support login mode '{:?}'",
                request.provider,
                request.mode
            );
        }

        let resolved = match request.mode {
            ProviderLoginMode::Browser => {
                let prepared = backend.prepare_browser_login()?;
                println!(
                    "Open this URL in your browser to continue:\n\n{}\n",
                    prepared.authorize_url
                );
                let code = backend
                    .wait_for_browser_callback(&prepared.session, Duration::from_secs(180))
                    .await
                    .or_else(|_| {
                        anyhow::bail!(
                            "Browser callback timed out. Retry with `agent-diva provider login {} --paste-code`.",
                            request.provider
                        )
                    })?;
                backend
                    .complete_browser_login(&code, &prepared.session)
                    .await?
            }
            ProviderLoginMode::PasteRedirect { input } => {
                let prepared = backend.prepare_browser_login()?;
                let Some(input) = input else {
                    println!(
                        "Open this URL in your browser and paste the final redirect URL:\n\n{}\n",
                        prepared.authorize_url
                    );
                    anyhow::bail!("Missing redirect input for paste flow");
                };
                backend
                    .complete_browser_login(&input, &prepared.session)
                    .await?
            }
            ProviderLoginMode::DeviceCode => {
                let pending = backend.start_device_code_login().await?;
                println!(
                    "Open {} and enter code {}",
                    pending
                        .verification_uri_complete
                        .clone()
                        .unwrap_or_else(|| pending.verification_uri.clone()),
                    pending.user_code
                );
                backend.poll_device_code_login(&pending).await?
            }
        };

        self.persist_resolved_login(auth_service, &request.profile_name, resolved)
            .await
    }

    fn backend(&self, provider: &str) -> Result<&dyn ProviderOAuthBackend> {
        match provider {
            "openai-codex" => Ok(&self.openai_codex),
            "qwen-login" => Ok(&self.qwen_login),
            _ => anyhow::bail!("Provider login is not implemented for '{provider}'"),
        }
    }

    async fn persist_resolved_login(
        &self,
        auth_service: &ProviderAuthService,
        profile_name: &str,
        resolved: ResolvedOAuthLogin,
    ) -> Result<ProviderLoginResult> {
        let profile = auth_service
            .store_oauth_profile(
                &resolved.provider,
                profile_name,
                OAuthProfileState {
                    token_set: resolved.token_set.clone(),
                    account_id: resolved.account_id.clone(),
                    metadata: resolved.metadata.clone(),
                },
                true,
            )
            .await?;
        Ok(ProviderLoginResult {
            provider: resolved.provider,
            profile_name: profile_name.to_string(),
            account_id: profile.account_id,
            status: "authenticated".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_login_requires_registry_and_backend() {
        let service = ProviderLoginService::new();
        assert!(service.supports_login("openai-codex"));
        assert!(service.supports_login("qwen-login"));
        assert!(!service.supports_login("dashscope"));
    }

    #[test]
    fn qwen_login_device_code_is_not_supported() {
        let service = ProviderLoginService::new();
        assert!(!service.supports_mode("qwen-login", &ProviderLoginMode::DeviceCode));
    }

    #[test]
    fn qwen_login_browser_flow_dispatches_to_qwen_backend() {
        let service = ProviderLoginService::new();
        let prepared = service.prepare_browser_login("qwen-login").unwrap();
        assert!(prepared.authorize_url.starts_with("https://chat.qwen.ai/"));
        assert!(!prepared.session.state.is_empty());
    }
}
