//! OS credential slots for speech providers. Keys live only in the
//! platform store, keyed by a FIXED namespace derived at startup — never
//! by arbitrary service/profile strings from the wire. Readback is
//! presence-only; an unavailable or locked store surfaces
//! `credential_unavailable` with no plaintext/env/sample fallback.

use keyring::Entry;

use crate::config::SpeechProvider;
use crate::{SpeechCode, SpeechError, SpeechResult};

/// Fixed keyring service namespace — one app, one profile.
const SERVICE: &str = "dev.projectivy.diva.speech";

fn slot(provider: SpeechProvider) -> String {
    format!("v1.{}", provider.as_str())
}

/// What a store may be; injected for tests.
pub trait SecretStore: Send + Sync {
    fn set(&self, service: &str, user: &str, secret: &str) -> SpeechResult<()>;
    fn get(&self, service: &str, user: &str) -> SpeechResult<Option<String>>;
    fn delete(&self, service: &str, user: &str) -> SpeechResult<()>;
}

fn map_store_error(e: keyring::Error) -> SpeechError {
    // NoDefaultStore / platform failures are environment facts — not
    // retryable inside this process; transient store errors are.
    let retryable = !matches!(e, keyring::Error::NoDefaultStore);
    SpeechError::new(
        SpeechCode::CredentialUnavailable,
        "OS credential store unavailable",
    )
    .retryable(retryable)
}

pub struct OsKeyring;

impl SecretStore for OsKeyring {
    fn set(&self, service: &str, user: &str, secret: &str) -> SpeechResult<()> {
        let entry = Entry::new(service, user).map_err(map_store_error)?;
        entry.set_password(secret).map_err(map_store_error)
    }

    fn get(&self, service: &str, user: &str) -> SpeechResult<Option<String>> {
        let entry = Entry::new(service, user).map_err(map_store_error)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(map_store_error(e)),
        }
    }

    fn delete(&self, service: &str, user: &str) -> SpeechResult<()> {
        let entry = Entry::new(service, user).map_err(map_store_error)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(map_store_error(e)),
        }
    }
}

/// Presence readback states — never the secret itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialPresence {
    Present,
    Absent,
    Unavailable,
}

pub struct CredentialStore<S: SecretStore = OsKeyring> {
    store: S,
}

impl CredentialStore<OsKeyring> {
    pub fn os() -> Self {
        Self { store: OsKeyring }
    }
}

impl<S: SecretStore> CredentialStore<S> {
    pub fn with_store(store: S) -> Self {
        Self { store }
    }

    /// Set a provider key; value never echoed anywhere. `key` is bounded
    /// and must be non-empty — it is still never logged or returned.
    pub fn set(&self, provider: SpeechProvider, key: &str) -> SpeechResult<()> {
        if key.trim().is_empty() || key.len() > 512 {
            return Err(SpeechError::new(
                SpeechCode::InvalidInput,
                "credential empty or overlong",
            ));
        }
        self.store
            .set(SERVICE, &slot(provider), key)
            .map_err(|e| e.provider(provider))
    }

    /// Presence-only readback for `speech_config_get` — no key material.
    pub fn presence(&self, provider: SpeechProvider) -> CredentialPresence {
        match self.store.get(SERVICE, &slot(provider)) {
            Ok(Some(_)) => CredentialPresence::Present,
            Ok(None) => CredentialPresence::Absent,
            Err(_) => CredentialPresence::Unavailable,
        }
    }

    /// Idempotent delete — absent is a success.
    pub fn delete(&self, provider: SpeechProvider) -> SpeechResult<()> {
        self.store
            .delete(SERVICE, &slot(provider))
            .map_err(|e| e.provider(provider))
    }
}
