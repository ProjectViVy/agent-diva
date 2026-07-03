//! Gateway identity tracking
//!
//! Provides `GatewayIdentity` to uniquely identify a gateway process
//! across restarts via a persistent boot UUID.

use chrono::{DateTime, Utc};
use std::path::Path;
use uuid::Uuid;

use crate::error::{Error, Result};

/// Identity of a single gateway process instance.
///
/// `boot_id` is a UUID persisted to disk so it survives restarts,
/// allowing the same gateway data directory to be recognised across reboots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayIdentity {
    /// Persistent boot identifier (survives restarts).
    pub boot_id: Uuid,
    /// OS process ID at the time the identity was created.
    pub pid: i32,
    /// Timestamp when this identity record was created.
    pub started_at: DateTime<Utc>,
}

impl GatewayIdentity {
    /// File name used to persist the boot UUID inside `data_root`.
    const BOOT_ID_FILE: &'static str = "gateway_boot_id";

    /// Create or load a `GatewayIdentity`.
    ///
    /// - If `data_root/gateway_boot_id` exists, reads the UUID from it.
    /// - Otherwise generates a new v4 UUID, writes it to the file, and returns it.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but contains an invalid UUID,
    /// or if I/O operations fail.
    pub fn new(data_root: &Path) -> Result<Self> {
        let boot_id_path = data_root.join(Self::BOOT_ID_FILE);

        let boot_id = if boot_id_path.exists() {
            let contents = std::fs::read_to_string(&boot_id_path).map_err(Error::Io)?;
            let trimmed = contents.trim();
            Uuid::parse_str(trimmed).map_err(|e| {
                Error::Validation(format!("Invalid boot_id UUID in {:?}: {}", boot_id_path, e))
            })?
        } else {
            let new_id = Uuid::new_v4();
            // Ensure parent directory exists
            if let Some(parent) = boot_id_path.parent() {
                std::fs::create_dir_all(parent).map_err(Error::Io)?;
            }
            std::fs::write(&boot_id_path, new_id.to_string()).map_err(Error::Io)?;
            new_id
        };

        Ok(Self {
            boot_id,
            pid: std::process::id() as i32,
            started_at: Utc::now(),
        })
    }

    /// Create a `GatewayIdentity` for the current process without persistence.
    ///
    /// Generates a fresh v4 UUID every time. Useful for ephemeral or test
    /// scenarios where persistence is not required.
    pub fn current() -> Self {
        Self {
            boot_id: Uuid::new_v4(),
            pid: std::process::id() as i32,
            started_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn gateway_identity_new_generates_and_persists_boot_id() {
        let temp = TempDir::new().unwrap();
        let data_root = temp.path();

        let id1 = GatewayIdentity::new(data_root).unwrap();
        assert_eq!(id1.pid, std::process::id() as i32);
        assert!(id1.started_at <= Utc::now());

        // Boot ID should have been persisted
        let boot_id_file = data_root.join("gateway_boot_id");
        assert!(boot_id_file.exists());
        let persisted = std::fs::read_to_string(&boot_id_file).unwrap();
        assert_eq!(persisted.trim(), id1.boot_id.to_string());

        // Second call should read the same boot_id
        let id2 = GatewayIdentity::new(data_root).unwrap();
        assert_eq!(id1.boot_id, id2.boot_id);
    }

    #[test]
    fn gateway_identity_current_generates_fresh_uuid() {
        let id1 = GatewayIdentity::current();
        let id2 = GatewayIdentity::current();

        assert_ne!(id1.boot_id, id2.boot_id);
        assert_eq!(id1.pid, std::process::id() as i32);
        assert_eq!(id2.pid, std::process::id() as i32);
    }

    #[test]
    fn gateway_identity_rejects_invalid_persisted_uuid() {
        let temp = TempDir::new().unwrap();
        let data_root = temp.path();
        let boot_id_file = data_root.join("gateway_boot_id");
        std::fs::write(&boot_id_file, "not-a-uuid").unwrap();

        let result = GatewayIdentity::new(data_root);
        assert!(result.is_err());
    }
}
