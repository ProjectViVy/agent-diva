//! Security policy configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Security level presets
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityLevel {
    /// Minimal restrictions (development only)
    Permissive,
    /// Standard restrictions with workspace limits (default)
    #[default]
    Standard,
    /// Strict mode with additional validation
    Strict,
    /// Read-only mode, no modifications allowed
    Paranoid,
}

impl SecurityLevel {
    /// Get default max actions per hour for this level
    pub fn default_max_actions_per_hour(&self) -> u32 {
        match self {
            Self::Permissive => 1000,
            Self::Standard => 100,
            Self::Strict => 50,
            Self::Paranoid => 20,
        }
    }

    /// Whether workspace_only is enforced by default
    pub fn default_workspace_only(&self) -> bool {
        match self {
            Self::Permissive => false,
            Self::Standard | Self::Strict | Self::Paranoid => true,
        }
    }

    /// Whether read-only mode is enabled
    pub fn is_read_only(&self) -> bool {
        matches!(self, Self::Paranoid)
    }
}

/// Security policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Security level preset
    pub level: SecurityLevel,

    /// Whether to restrict operations to workspace only
    pub workspace_only: bool,

    /// Maximum file operations per hour
    pub max_actions_per_hour: u32,

    /// Forbidden path prefixes
    pub forbidden_paths: Vec<String>,

    /// Additional allowed roots outside workspace
    pub allowed_roots: Vec<PathBuf>,

    /// Forbidden file extensions
    pub forbidden_extensions: Vec<String>,

    /// Read-only mode (overrides level setting)
    pub read_only: Option<bool>,

    /// Maximum file size in bytes (0 = unlimited)
    pub max_file_size: u64,

    /// Enable symlink following
    pub allow_symlinks: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            level: SecurityLevel::default(),
            workspace_only: true,
            max_actions_per_hour: 100,
            forbidden_paths: vec![
                "/etc".to_string(),
                "/root".to_string(),
                "/sys".to_string(),
                "/proc".to_string(),
                "~/.ssh".to_string(),
                "~/.gnupg".to_string(),
                "~/.aws".to_string(),
            ],
            allowed_roots: Vec::new(),
            forbidden_extensions: vec![
                ".exe".to_string(),
                ".dll".to_string(),
                ".bat".to_string(),
                ".cmd".to_string(),
                ".sh".to_string(),
            ],
            read_only: None,
            max_file_size: 10 * 1024 * 1024, // 10MB
            allow_symlinks: false,
        }
    }
}

impl SecurityConfig {
    /// Create config from security level
    pub fn from_level(level: SecurityLevel) -> Self {
        Self {
            level,
            workspace_only: level.default_workspace_only(),
            max_actions_per_hour: level.default_max_actions_per_hour(),
            read_only: Some(level.is_read_only()),
            ..Self::default()
        }
    }

    /// Check if read-only mode is enabled
    pub fn is_read_only(&self) -> bool {
        self.read_only.unwrap_or_else(|| self.level.is_read_only())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_actions_per_hour == 0 {
            return Err("max_actions_per_hour must be greater than 0".to_string());
        }

        for path in &self.forbidden_paths {
            if path.contains('\0') {
                return Err(format!("Forbidden path contains null byte: {}", path));
            }
        }

        Ok(())
    }

    /// Merge with another config (other takes precedence for non-default values)
    pub fn merge(&mut self, other: SecurityConfig) {
        if other.level != SecurityLevel::default() {
            self.level = other.level;
        }
        if !other.forbidden_paths.is_empty() {
            self.forbidden_paths = other.forbidden_paths;
        }
        if !other.allowed_roots.is_empty() {
            self.allowed_roots = other.allowed_roots;
        }
        if other.max_file_size != 0 {
            self.max_file_size = other.max_file_size;
        }
        if other.read_only.is_some() {
            self.read_only = other.read_only;
        }
        // Note: max_actions_per_hour is set via level in SecurityConfig::from_level,
        // but we also check for non-default values in direct merge
        if other.max_actions_per_hour != SecurityLevel::default().default_max_actions_per_hour() {
            self.max_actions_per_hour = other.max_actions_per_hour;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_level_defaults() {
        assert_eq!(
            SecurityLevel::Permissive.default_max_actions_per_hour(),
            1000
        );
        assert_eq!(SecurityLevel::Standard.default_max_actions_per_hour(), 100);
        assert_eq!(SecurityLevel::Strict.default_max_actions_per_hour(), 50);
        assert_eq!(SecurityLevel::Paranoid.default_max_actions_per_hour(), 20);

        assert!(!SecurityLevel::Permissive.default_workspace_only());
        assert!(SecurityLevel::Standard.default_workspace_only());
        assert!(SecurityLevel::Paranoid.is_read_only());
    }

    #[test]
    fn test_config_from_level() {
        let config = SecurityConfig::from_level(SecurityLevel::Strict);
        assert_eq!(config.max_actions_per_hour, 50);
        assert!(config.workspace_only);
        assert!(!config.is_read_only());

        let config = SecurityConfig::from_level(SecurityLevel::Paranoid);
        assert!(config.is_read_only());
    }

    #[test]
    fn test_config_validation() {
        let mut config = SecurityConfig::default();
        assert!(config.validate().is_ok());

        config.max_actions_per_hour = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_merge() {
        let mut base = SecurityConfig::default();
        let other = SecurityConfig {
            level: SecurityLevel::Strict,
            max_actions_per_hour: 50,
            ..Default::default()
        };

        base.merge(other);
        assert_eq!(base.level, SecurityLevel::Strict);
        assert_eq!(base.max_actions_per_hour, 50);
    }
}
