use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

pub const CURRENT_AUTH_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAuthKind {
    OAuth,
    Token,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderTokenSet {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

impl ProviderTokenSet {
    pub fn is_expiring_within(&self, skew: Duration) -> bool {
        match self.expires_at {
            Some(expires_at) => {
                let Ok(skew) = chrono::Duration::from_std(skew) else {
                    return false;
                };
                expires_at <= Utc::now() + skew
            }
            None => false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderAuthProfile {
    pub id: String,
    pub provider: String,
    pub profile_name: String,
    pub kind: ProviderAuthKind,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub token_set: Option<ProviderTokenSet>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for ProviderAuthProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderAuthProfile")
            .field("id", &self.id)
            .field("provider", &self.provider)
            .field("profile_name", &self.profile_name)
            .field("kind", &self.kind)
            .field("account_id", &self.account_id)
            .field("metadata", &self.metadata)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish_non_exhaustive()
    }
}

impl ProviderAuthProfile {
    pub fn new_oauth(
        provider: impl Into<String>,
        profile_name: impl Into<String>,
        token_set: ProviderTokenSet,
    ) -> Self {
        let provider = provider.into();
        let profile_name = profile_name.into();
        let now = Utc::now();
        Self {
            id: profile_id(&provider, &profile_name),
            provider,
            profile_name,
            kind: ProviderAuthKind::OAuth,
            account_id: None,
            token_set: Some(token_set),
            token: None,
            metadata: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_token(
        provider: impl Into<String>,
        profile_name: impl Into<String>,
        token: impl Into<String>,
    ) -> Self {
        let provider = provider.into();
        let profile_name = profile_name.into();
        let now = Utc::now();
        Self {
            id: profile_id(&provider, &profile_name),
            provider,
            profile_name,
            kind: ProviderAuthKind::Token,
            account_id: None,
            token_set: None,
            token: Some(token.into()),
            metadata: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderAuthProfilesData {
    pub schema_version: u32,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub active_profiles: BTreeMap<String, String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, ProviderAuthProfile>,
}

impl Default for ProviderAuthProfilesData {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_AUTH_SCHEMA_VERSION,
            updated_at: Utc::now(),
            active_profiles: BTreeMap::new(),
            profiles: BTreeMap::new(),
        }
    }
}

pub fn profile_id(provider: &str, profile_name: &str) -> String {
    format!("{provider}:{profile_name}")
}
