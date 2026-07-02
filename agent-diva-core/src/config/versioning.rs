//! Configuration version tracking and migration facade
//!
//! Thin facade that delegates to the migration infrastructure.
//! The actual migration logic lives in agent-diva-migration;
//! this module provides version types and a dispatch point.
//!
//! Because agent-diva-migration depends on agent-diva-core (not vice versa),
//! the migrate function uses an injectable strategy pattern. The CLI layer
//! wires the real ConfigMigrator at startup.

use std::path::Path;
use std::sync::OnceLock;

/// Known configuration schema versions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigVersion {
    /// Original Python config format
    V1,
    /// Current Rust config format
    V2,
    /// Unknown version (forward-compatible)
    Unknown(String),
}

impl ConfigVersion {
    /// Human-readable version label
    pub fn as_str(&self) -> &str {
        match self {
            ConfigVersion::V1 => "v1",
            ConfigVersion::V2 => "v2",
            ConfigVersion::Unknown(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for ConfigVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Return the current config schema version
pub fn current_version() -> ConfigVersion {
    ConfigVersion::V2
}

/// Type-erased migration strategy.
///
/// The CLI layer injects the real `agent_diva_migration::ConfigMigrator`
/// at startup so core does not depend on the migration crate directly.
pub type MigrateFn = dyn Fn(&Path, &Path, bool) -> anyhow::Result<bool> + Send + Sync;

static MIGRATE_STRATEGY: OnceLock<Box<MigrateFn>> = OnceLock::new();

/// Register the migration strategy (called once at startup from CLI layer).
///
/// # Panics
///
/// Panics if called more than once.
pub fn set_migrate_strategy(f: Box<MigrateFn>) {
    if MIGRATE_STRATEGY.set(f).is_err() {
        panic!("migrate strategy already registered");
    }
}

/// Migrate configuration from source_dir to target_dir.
///
/// Delegates to the registered migration strategy.
/// Returns true if migration was performed.
///
/// # Errors
///
/// Returns an error if no migrate strategy has been registered.
pub fn migrate(
    source_dir: impl AsRef<Path>,
    target_dir: impl AsRef<Path>,
    dry_run: bool,
) -> anyhow::Result<bool> {
    let strategy = MIGRATE_STRATEGY
        .get()
        .ok_or_else(|| anyhow::anyhow!("migrate strategy not registered — call set_migrate_strategy at startup"))?;
    strategy(source_dir.as_ref(), target_dir.as_ref(), dry_run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_version_is_v2() {
        assert_eq!(current_version(), ConfigVersion::V2);
    }

    #[test]
    fn test_version_variants() {
        let v1 = ConfigVersion::V1;
        let v2 = ConfigVersion::V2;
        let unknown = ConfigVersion::Unknown("v3".to_string());

        assert_eq!(v1.as_str(), "v1");
        assert_eq!(v2.as_str(), "v2");
        assert_eq!(unknown.as_str(), "v3");
        assert_eq!(format!("{}", v1), "v1");
        assert_eq!(format!("{}", unknown), "v3");
    }

    #[test]
    fn test_version_equality() {
        assert_eq!(ConfigVersion::V1, ConfigVersion::V1);
        assert_ne!(ConfigVersion::V1, ConfigVersion::V2);
        assert_eq!(
            ConfigVersion::Unknown("v3".into()),
            ConfigVersion::Unknown("v3".into())
        );
        assert_ne!(
            ConfigVersion::Unknown("v3".into()),
            ConfigVersion::Unknown("v4".into())
        );
    }

    #[test]
    fn test_version_clone() {
        let v = ConfigVersion::V2;
        assert_eq!(v.clone(), v);
    }

    #[test]
    fn test_migrate_without_strategy_errors() {
        // Strategy not set → error
        let result = migrate("/tmp/src", "/tmp/tgt", true);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("strategy not registered"));
    }

    #[test]
    fn test_migrate_type_signatures() {
        // Verify MigrateFn type compiles and can be constructed
        fn _assert_migrate_fn_works(f: Box<MigrateFn>) {
            let result = f(std::path::Path::new("/src"), std::path::Path::new("/tgt"), true);
            let _ = result;
        }
        _assert_migrate_fn_works(Box::new(|_, _, _| Ok(true)));
    }
}
