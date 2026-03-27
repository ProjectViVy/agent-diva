use crate::auth::profiles::{
    profile_id, ProviderAuthKind, ProviderAuthProfile, ProviderAuthProfilesData, ProviderTokenSet,
};
use crate::auth::store::ProviderAuthStore;
use anyhow::{Context, Result};
use base64::Engine;
use chrono::Utc;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

const DEFAULT_PROFILE_NAME: &str = "default";
const OPENAI_CODEX_PROVIDER: &str = "openai-codex";
const OPENAI_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OPENAI_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OPENAI_REFRESH_SKEW_SECS: u64 = 90;

#[derive(Clone)]
pub struct ProviderAuthService {
    store: ProviderAuthStore,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct OpenAiTokenResponse {
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

impl ProviderAuthService {
    pub fn new(config_dir: &Path) -> Self {
        Self {
            store: ProviderAuthStore::new(config_dir),
            client: reqwest::Client::new(),
        }
    }

    pub fn store(&self) -> &ProviderAuthStore {
        &self.store
    }

    pub async fn load_profiles(&self) -> Result<ProviderAuthProfilesData> {
        self.store.load().await
    }

    pub async fn store_openai_codex_tokens(
        &self,
        profile_name: &str,
        token_set: ProviderTokenSet,
    ) -> Result<ProviderAuthProfile> {
        let mut profile =
            ProviderAuthProfile::new_oauth(OPENAI_CODEX_PROVIDER, profile_name, token_set.clone());
        profile.account_id = extract_account_id_from_jwt(&token_set.access_token);
        self.store.upsert_profile(profile.clone(), true).await?;
        Ok(profile)
    }

    pub async fn get_profile(
        &self,
        provider: &str,
        profile_override: Option<&str>,
    ) -> Result<Option<ProviderAuthProfile>> {
        let data = self.store.load().await?;
        let Some(id) = select_profile_id(&data, provider, profile_override) else {
            return Ok(None);
        };
        Ok(data.profiles.get(&id).cloned())
    }

    pub async fn get_active_profile(&self, provider: &str) -> Result<Option<ProviderAuthProfile>> {
        self.get_profile(provider, None).await
    }

    pub async fn get_provider_bearer_token(
        &self,
        provider: &str,
        profile_override: Option<&str>,
    ) -> Result<Option<String>> {
        let profile = self.get_profile(provider, profile_override).await?;
        let Some(profile) = profile else {
            return Ok(None);
        };
        Ok(match profile.kind {
            ProviderAuthKind::OAuth => profile.token_set.and_then(|token_set| {
                (!token_set.access_token.trim().is_empty()).then_some(token_set.access_token)
            }),
            ProviderAuthKind::Token => profile
                .token
                .and_then(|token| (!token.trim().is_empty()).then_some(token)),
        })
    }

    pub async fn get_valid_openai_codex_access_token(
        &self,
        profile_override: Option<&str>,
    ) -> Result<Option<String>> {
        let profile = self
            .get_profile(OPENAI_CODEX_PROVIDER, profile_override)
            .await?;
        let Some(profile) = profile else {
            return Ok(None);
        };
        let Some(token_set) = profile.token_set.clone() else {
            anyhow::bail!("OpenAI Codex auth profile is missing OAuth token set");
        };

        if !token_set.is_expiring_within(Duration::from_secs(OPENAI_REFRESH_SKEW_SECS)) {
            return Ok(Some(token_set.access_token));
        }

        let Some(refresh_token) = token_set.refresh_token.clone() else {
            return Ok(Some(token_set.access_token));
        };
        let refreshed = self
            .refresh_openai_codex_tokens_with_refresh_token(&refresh_token)
            .await?;
        let access_token = refreshed.access_token.clone();
        let account_id = extract_account_id_from_jwt(&access_token);
        let profile_id = profile.id.clone();
        self.store
            .update_profile(&profile_id, |profile| {
                profile.token_set = Some(refreshed.clone());
                profile.account_id = account_id.clone();
                Ok(())
            })
            .await?;
        Ok(Some(access_token))
    }

    pub async fn refresh_openai_codex_tokens(
        &self,
        profile_override: Option<&str>,
    ) -> Result<ProviderAuthProfile> {
        let profile = self
            .get_profile(OPENAI_CODEX_PROVIDER, profile_override)
            .await?
            .ok_or_else(|| anyhow::anyhow!("OpenAI Codex auth profile not found"))?;
        let refresh_token = profile
            .token_set
            .as_ref()
            .and_then(|tokens| tokens.refresh_token.clone())
            .ok_or_else(|| {
                anyhow::anyhow!("OpenAI Codex auth profile does not contain a refresh token")
            })?;
        let refreshed = self
            .refresh_openai_codex_tokens_with_refresh_token(&refresh_token)
            .await?;
        let account_id = extract_account_id_from_jwt(&refreshed.access_token);
        self.store
            .update_profile(&profile.id, |existing| {
                existing.token_set = Some(refreshed.clone());
                existing.account_id = account_id.clone();
                Ok(())
            })
            .await
    }

    pub async fn set_active_profile(&self, provider: &str, profile_name: &str) -> Result<String> {
        let requested_id = if profile_name.contains(':') {
            profile_name.to_string()
        } else {
            profile_id(provider, profile_name)
        };
        self.store
            .set_active_profile(provider, &requested_id)
            .await?;
        Ok(requested_id)
    }

    pub async fn remove_profile(&self, provider: &str, profile_name: &str) -> Result<bool> {
        let requested_id = if profile_name.contains(':') {
            profile_name.to_string()
        } else {
            profile_id(provider, profile_name)
        };
        self.store.remove_profile(&requested_id).await
    }

    async fn refresh_openai_codex_tokens_with_refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<ProviderTokenSet> {
        let response = self
            .client
            .post(OPENAI_OAUTH_TOKEN_URL)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", OPENAI_OAUTH_CLIENT_ID),
            ])
            .send()
            .await
            .context("Failed to refresh OpenAI Codex OAuth token")?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI Codex token refresh failed ({status}): {body}");
        }
        let parsed: OpenAiTokenResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI Codex token refresh response")?;
        let expires_at = parsed
            .expires_in
            .map(|seconds| Utc::now() + chrono::Duration::seconds(seconds));
        Ok(ProviderTokenSet {
            access_token: parsed.access_token,
            refresh_token: parsed
                .refresh_token
                .or_else(|| Some(refresh_token.to_string())),
            id_token: parsed.id_token,
            expires_at,
            token_type: parsed.token_type,
            scope: parsed.scope,
        })
    }
}

fn select_profile_id(
    data: &ProviderAuthProfilesData,
    provider: &str,
    profile_override: Option<&str>,
) -> Option<String> {
    profile_override
        .map(|value| {
            if value.contains(':') {
                value.to_string()
            } else {
                profile_id(provider, value)
            }
        })
        .or_else(|| data.active_profiles.get(provider).cloned())
        .or_else(|| {
            let fallback = profile_id(provider, DEFAULT_PROFILE_NAME);
            data.profiles.contains_key(&fallback).then_some(fallback)
        })
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
    use tempfile::tempdir;

    #[tokio::test]
    async fn store_and_get_bearer_token() {
        let dir = tempdir().unwrap();
        let service = ProviderAuthService::new(dir.path());
        service
            .store_openai_codex_tokens(
                "default",
                ProviderTokenSet {
                    access_token: "access".into(),
                    refresh_token: Some("refresh".into()),
                    id_token: None,
                    expires_at: None,
                    token_type: Some("Bearer".into()),
                    scope: Some("openid".into()),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            service
                .get_provider_bearer_token("openai-codex", None)
                .await
                .unwrap()
                .as_deref(),
            Some("access")
        );
    }

    #[tokio::test]
    async fn set_active_profile_uses_profile_name() {
        let dir = tempdir().unwrap();
        let service = ProviderAuthService::new(dir.path());
        service
            .store_openai_codex_tokens(
                "work",
                ProviderTokenSet {
                    access_token: "access".into(),
                    refresh_token: Some("refresh".into()),
                    id_token: None,
                    expires_at: None,
                    token_type: None,
                    scope: None,
                },
            )
            .await
            .unwrap();
        let selected = service
            .set_active_profile("openai-codex", "work")
            .await
            .unwrap();
        assert_eq!(selected, "openai-codex:work");
    }
}
