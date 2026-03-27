mod openai_codex;

use crate::registry::{AuthMode, ProviderRegistry};
use agent_diva_core::auth::ProviderAuthService;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub use openai_codex::{OpenAiCodexAuthHandler, OpenAiCodexBrowserSession};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderLoginRequest {
    pub provider: String,
    pub profile_name: String,
    pub mode: ProviderLoginMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Clone, Default)]
pub struct ProviderLoginService {
    registry: ProviderRegistry,
    openai_codex: OpenAiCodexAuthHandler,
}

impl ProviderLoginService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn supports_login(&self, provider: &str) -> bool {
        self.registry
            .find_by_name(provider)
            .is_some_and(|spec| spec.login_supported && spec.auth_mode == AuthMode::OAuth)
    }

    pub async fn login(
        &self,
        auth_service: &ProviderAuthService,
        request: ProviderLoginRequest,
    ) -> Result<ProviderLoginResult> {
        match request.provider.as_str() {
            "openai-codex" => self.openai_codex.login(auth_service, request).await,
            provider => anyhow::bail!("Provider login is not implemented for '{provider}'"),
        }
    }
}
