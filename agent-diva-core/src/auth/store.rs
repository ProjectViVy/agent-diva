use crate::auth::profiles::{ProviderAuthProfile, ProviderAuthProfilesData};
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::time::{sleep, Instant};

const AUTH_DIR: &str = "data/auth";
const AUTH_FILENAME: &str = "profiles.json";
const LOCK_FILENAME: &str = "profiles.lock";
const LOCK_RETRY_MS: u64 = 50;
const LOCK_TIMEOUT_MS: u64 = 10_000;

#[derive(Debug, Clone)]
pub struct ProviderAuthStore {
    path: PathBuf,
    lock_path: PathBuf,
}

impl ProviderAuthStore {
    pub fn new(config_dir: &Path) -> Self {
        let auth_dir = config_dir.join(AUTH_DIR);
        Self {
            path: auth_dir.join(AUTH_FILENAME),
            lock_path: auth_dir.join(LOCK_FILENAME),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn load(&self) -> Result<ProviderAuthProfilesData> {
        let _lock = self.acquire_lock().await?;
        self.load_locked().await
    }

    pub async fn upsert_profile(
        &self,
        mut profile: ProviderAuthProfile,
        set_active: bool,
    ) -> Result<()> {
        let _lock = self.acquire_lock().await?;
        let mut data = self.load_locked().await?;

        profile.updated_at = Utc::now();
        if let Some(existing) = data.profiles.get(&profile.id) {
            profile.created_at = existing.created_at;
        }

        if set_active {
            data.active_profiles
                .insert(profile.provider.clone(), profile.id.clone());
        }
        data.profiles.insert(profile.id.clone(), profile);
        data.updated_at = Utc::now();
        self.save_locked(&data).await
    }

    pub async fn remove_profile(&self, profile_id: &str) -> Result<bool> {
        let _lock = self.acquire_lock().await?;
        let mut data = self.load_locked().await?;
        let removed = data.profiles.remove(profile_id).is_some();
        if removed {
            data.active_profiles.retain(|_, id| id != profile_id);
            data.updated_at = Utc::now();
            self.save_locked(&data).await?;
        }
        Ok(removed)
    }

    pub async fn set_active_profile(&self, provider: &str, profile_id: &str) -> Result<()> {
        let _lock = self.acquire_lock().await?;
        let mut data = self.load_locked().await?;
        if !data.profiles.contains_key(profile_id) {
            anyhow::bail!("Auth profile not found: {profile_id}");
        }
        data.active_profiles
            .insert(provider.to_string(), profile_id.to_string());
        data.updated_at = Utc::now();
        self.save_locked(&data).await
    }

    pub async fn clear_active_profile(&self, provider: &str) -> Result<()> {
        let _lock = self.acquire_lock().await?;
        let mut data = self.load_locked().await?;
        data.active_profiles.remove(provider);
        data.updated_at = Utc::now();
        self.save_locked(&data).await
    }

    pub async fn update_profile<F>(
        &self,
        profile_id: &str,
        mut updater: F,
    ) -> Result<ProviderAuthProfile>
    where
        F: FnMut(&mut ProviderAuthProfile) -> Result<()>,
    {
        let _lock = self.acquire_lock().await?;
        let mut data = self.load_locked().await?;
        let profile = data
            .profiles
            .get_mut(profile_id)
            .ok_or_else(|| anyhow::anyhow!("Auth profile not found: {profile_id}"))?;
        updater(profile)?;
        profile.updated_at = Utc::now();
        let updated = profile.clone();
        data.updated_at = Utc::now();
        self.save_locked(&data).await?;
        Ok(updated)
    }

    async fn load_locked(&self) -> Result<ProviderAuthProfilesData> {
        if !self.path.exists() {
            return Ok(ProviderAuthProfilesData::default());
        }
        let raw = fs::read_to_string(&self.path)
            .await
            .with_context(|| format!("Failed to read auth store {}", self.path.display()))?;
        let data = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse auth store {}", self.path.display()))?;
        Ok(data)
    }

    async fn save_locked(&self, data: &ProviderAuthProfilesData) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create auth store directory {}", parent.display())
            })?;
        }
        let temp_path = self.path.with_extension("json.tmp");
        let payload = serde_json::to_vec_pretty(data)?;
        fs::write(&temp_path, payload)
            .await
            .with_context(|| format!("Failed to write auth temp file {}", temp_path.display()))?;
        fs::rename(&temp_path, &self.path).await.with_context(|| {
            format!(
                "Failed to move auth temp file {} into {}",
                temp_path.display(),
                self.path.display()
            )
        })?;
        Ok(())
    }

    async fn acquire_lock(&self) -> Result<LockGuard> {
        if let Some(parent) = self.lock_path.parent() {
            fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create auth lock directory {}", parent.display())
            })?;
        }

        let deadline = Instant::now() + Duration::from_millis(LOCK_TIMEOUT_MS);
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&self.lock_path)
                .await
            {
                Ok(_) => return Ok(LockGuard(self.lock_path.clone())),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    if Instant::now() >= deadline {
                        anyhow::bail!("Timed out acquiring auth store lock");
                    }
                    sleep(Duration::from_millis(LOCK_RETRY_MS)).await;
                }
                Err(err) => {
                    return Err(err).with_context(|| {
                        format!("Failed to open auth lock {}", self.lock_path.display())
                    })
                }
            }
        }
    }
}

struct LockGuard(PathBuf);

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::profiles::{
        profile_id, ProviderAuthKind, ProviderAuthProfile, ProviderTokenSet,
    };
    use tempfile::tempdir;

    fn oauth_profile() -> ProviderAuthProfile {
        ProviderAuthProfile::new_oauth(
            "openai-codex",
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
    }

    #[tokio::test]
    async fn upsert_load_remove_profile_roundtrip() {
        let dir = tempdir().unwrap();
        let store = ProviderAuthStore::new(dir.path());
        store.upsert_profile(oauth_profile(), true).await.unwrap();

        let loaded = store.load().await.unwrap();
        assert_eq!(
            loaded.active_profiles.get("openai-codex").unwrap(),
            "openai-codex:default"
        );
        assert!(loaded.profiles.contains_key("openai-codex:default"));

        let removed = store.remove_profile("openai-codex:default").await.unwrap();
        assert!(removed);
        let loaded = store.load().await.unwrap();
        assert!(!loaded.profiles.contains_key("openai-codex:default"));
    }

    #[tokio::test]
    async fn set_and_clear_active_profile() {
        let dir = tempdir().unwrap();
        let store = ProviderAuthStore::new(dir.path());
        store.upsert_profile(oauth_profile(), false).await.unwrap();
        store
            .set_active_profile("openai-codex", &profile_id("openai-codex", "default"))
            .await
            .unwrap();
        assert_eq!(
            store
                .load()
                .await
                .unwrap()
                .active_profiles
                .get("openai-codex")
                .cloned(),
            Some("openai-codex:default".into())
        );
        store.clear_active_profile("openai-codex").await.unwrap();
        assert!(!store
            .load()
            .await
            .unwrap()
            .active_profiles
            .contains_key("openai-codex"));
    }

    #[tokio::test]
    async fn update_profile_changes_token() {
        let dir = tempdir().unwrap();
        let store = ProviderAuthStore::new(dir.path());
        store.upsert_profile(oauth_profile(), true).await.unwrap();
        let updated = store
            .update_profile("openai-codex:default", |profile| {
                profile.kind = ProviderAuthKind::Token;
                profile.token_set = None;
                profile.token = Some("plain".into());
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(updated.token.as_deref(), Some("plain"));
    }

    #[tokio::test]
    async fn damaged_file_returns_error() {
        let dir = tempdir().unwrap();
        let store = ProviderAuthStore::new(dir.path());
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(store.path(), "{not-json").unwrap();
        let err = store.load().await.unwrap_err().to_string();
        assert!(err.contains("Failed to parse auth store"));
    }
}
