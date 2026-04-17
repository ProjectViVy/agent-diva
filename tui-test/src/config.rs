//! Configuration persistence module for TUI application.
//!
//! Handles loading and saving user preferences including language settings.

use crate::i18n::Language;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

/// Application configuration stored persistently
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// Current display language preference
    #[serde(default)]
    pub language: Language,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: Language::English,
        }
    }
}

impl AppConfig {
    /// Load configuration from disk, or return default if not found
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<AppConfig>(&content) {
                        Ok(config) => {
                            return config;
                        }
                        Err(e) => {
                            // Config file corrupted, log warning and use default
                            eprintln!("Warning: Failed to parse config file: {}", e);
                        }
                    }
                }
                Err(e) => {
                    // Failed to read config file
                    eprintln!("Warning: Failed to read config file: {}", e);
                }
            }
        }
        Self::default()
    }

    /// Save configuration to disk
    pub fn save(&self) -> io::Result<()> {
        let path = Self::config_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Get the configuration file path
    fn config_path() -> PathBuf {
        // Windows: %LOCALAPPDATA%\AgentDiVA-tui\config.json
        // Linux/Mac: ~/.config/AgentDiVA-tui/config.json
        #[cfg(target_os = "windows")]
        {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                return PathBuf::from(local_app_data)
                    .join("AgentDiVA-tui")
                    .join("config.json");
            }
            // Fallback
            PathBuf::from(".AgentDiVA-tui-config.json")
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home)
                    .join(".config")
                    .join("AgentDiVA-tui")
                    .join("config.json");
            }
            // Fallback
            PathBuf::from(".AgentDiVA-tui-config.json")
        }
    }

    /// Get the path string for display purposes
    pub fn config_path_display() -> String {
        Self::config_path().to_string_lossy().to_string()
    }

    /// Update language setting
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }
}

/// Implement Serde serialization for Language enum
impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Language::from_code(&s))
    }
}