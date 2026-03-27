use base64::Engine;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::auth::profiles::ProviderTokenSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkceState {
    pub state: String,
    pub code_verifier: String,
    pub code_challenge: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthProfileState {
    pub token_set: ProviderTokenSet,
    pub account_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

#[async_trait::async_trait]
pub trait OAuthTokenManager: Send + Sync {
    async fn refresh_oauth_state(&self, refresh_token: &str) -> anyhow::Result<OAuthProfileState>;

    fn extract_account_id(&self, access_token: &str) -> Option<String>;
}

pub fn generate_pkce_state() -> PkceState {
    let state = uuid::Uuid::new_v4().to_string();
    let code_verifier = format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let digest = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    PkceState {
        state,
        code_verifier,
        code_challenge,
    }
}

pub fn parse_code_from_redirect(
    input: &str,
    expected_state: Option<&str>,
) -> anyhow::Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        anyhow::bail!("OAuth redirect does not contain authorization code");
    }
    let query = trimmed
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or(trimmed);
    let params = parse_query_params(query);
    let callback_like = trimmed.contains('?')
        || params.contains_key("code")
        || params.contains_key("state")
        || params.contains_key("error");

    if let Some(error) = params.get("error") {
        anyhow::bail!("OAuth redirect returned error: {error}");
    }

    if let Some(state) = expected_state {
        if let Some(returned) = params.get("state") {
            if returned != state {
                anyhow::bail!("OAuth state mismatch");
            }
        } else if callback_like {
            anyhow::bail!("OAuth state mismatch");
        }
    }

    if let Some(code) = params.get("code").cloned() {
        return Ok(code);
    }
    if !callback_like {
        return Ok(trimmed.to_string());
    }
    anyhow::bail!("OAuth redirect does not contain authorization code")
}

pub fn parse_query_params(input: &str) -> BTreeMap<String, String> {
    input
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
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_state_contains_all_fields() {
        let state = generate_pkce_state();
        assert!(!state.state.is_empty());
        assert!(!state.code_verifier.is_empty());
        assert!(!state.code_challenge.is_empty());
    }

    #[test]
    fn parse_redirect_code_roundtrip() {
        let parsed =
            parse_code_from_redirect("/auth/callback?code=abc&state=expected", Some("expected"))
                .unwrap();
        assert_eq!(parsed, "abc");
    }

    #[test]
    fn parse_redirect_code_rejects_bad_state() {
        let err = parse_code_from_redirect("/auth/callback?code=abc&state=wrong", Some("expected"))
            .unwrap_err();
        assert!(err.to_string().contains("state mismatch"));
    }
}
