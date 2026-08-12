//! Persistent, conservatively validated command rules.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafePrefixSuggestion {
    pub pattern: Vec<String>,
    pub justification: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRuleSource {
    Legacy,
    Approval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRule {
    pub id: String,
    pub pattern: Vec<String>,
    #[serde(default = "default_decision")]
    pub decision: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_source")]
    pub source: CommandRuleSource,
    #[serde(default)]
    pub justification: String,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_revision")]
    pub revision: u64,
}

fn default_decision() -> String {
    "allow".into()
}
fn default_enabled() -> bool {
    true
}
fn default_source() -> CommandRuleSource {
    CommandRuleSource::Legacy
}
fn default_revision() -> u64 {
    1
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CommandRuleFile {
    #[serde(default = "file_version")]
    version: u32,
    #[serde(default, alias = "prefix_rules")]
    rules: Vec<CommandRule>,
}

fn file_version() -> u32 {
    2
}

#[derive(Debug, Error)]
pub enum CommandRuleError {
    #[error("command rule file is invalid: {0}")]
    InvalidFile(String),
    #[error("failed to persist command rules: {0}")]
    Persistence(String),
    #[error("command rule not found")]
    NotFound,
    #[error("command rule revision conflict")]
    RevisionConflict,
    #[error("command is not eligible for a persistent rule")]
    UnsafeSuggestion,
}

pub struct CommandRuleStore {
    path: PathBuf,
    rules: RwLock<Vec<CommandRule>>,
}

impl CommandRuleStore {
    pub fn open(path: PathBuf) -> Result<Self, CommandRuleError> {
        let rules = if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|error| CommandRuleError::InvalidFile(error.to_string()))?;
            parse_rule_file(&content)?
        } else {
            Vec::new()
        };
        Ok(Self {
            path,
            rules: RwLock::new(rules),
        })
    }

    pub fn list(&self) -> Vec<CommandRule> {
        self.rules.read().clone()
    }

    /// The path this store persists to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn allows(&self, command: &str) -> bool {
        let Ok(tokens) = shell_words::split(command) else {
            return false;
        };
        self.allows_tokens(&tokens)
    }

    /// Whether any enabled Allow rule exactly matches the given command tokens.
    pub fn allows_tokens(&self, tokens: &[String]) -> bool {
        self.rules
            .read()
            .iter()
            .any(|rule| rule.enabled && rule.decision == "allow" && rule.pattern == *tokens)
    }

    pub fn add_suggestion(
        &self,
        suggestion: &SafePrefixSuggestion,
    ) -> Result<CommandRule, CommandRuleError> {
        if safe_prefix_suggestion(&suggestion.pattern.join(" ")).as_ref() != Some(suggestion) {
            return Err(CommandRuleError::UnsafeSuggestion);
        }
        let mut next = self.rules.read().clone();
        if let Some(existing) = next.iter().find(|rule| rule.pattern == suggestion.pattern) {
            return Ok(existing.clone());
        }
        let rule = CommandRule {
            id: Uuid::new_v4().to_string(),
            pattern: suggestion.pattern.clone(),
            decision: "allow".into(),
            enabled: true,
            source: CommandRuleSource::Approval,
            justification: suggestion.justification.clone(),
            created_at: Utc::now(),
            revision: 1,
        };
        next.push(rule.clone());
        self.persist_and_replace(next)?;
        emit_rule_audit(&rule.id, "command_rule_created", &rule.pattern);
        Ok(rule)
    }

    pub fn set_enabled(
        &self,
        id: &str,
        revision: u64,
        enabled: bool,
    ) -> Result<CommandRule, CommandRuleError> {
        let mut next = self.rules.read().clone();
        let rule = next
            .iter_mut()
            .find(|rule| rule.id == id)
            .ok_or(CommandRuleError::NotFound)?;
        if rule.revision != revision {
            return Err(CommandRuleError::RevisionConflict);
        }
        rule.enabled = enabled;
        rule.revision += 1;
        let updated = rule.clone();
        self.persist_and_replace(next)?;
        emit_rule_audit(&updated.id, "command_rule_updated", &updated.pattern);
        Ok(updated)
    }

    pub fn delete(&self, id: &str, revision: u64) -> Result<(), CommandRuleError> {
        let mut next = self.rules.read().clone();
        let index = next
            .iter()
            .position(|rule| rule.id == id)
            .ok_or(CommandRuleError::NotFound)?;
        if next[index].revision != revision {
            return Err(CommandRuleError::RevisionConflict);
        }
        let removed = next.remove(index);
        self.persist_and_replace(next)?;
        emit_rule_audit(&removed.id, "command_rule_deleted", &removed.pattern);
        Ok(())
    }

    fn persist_and_replace(&self, next: Vec<CommandRule>) -> Result<(), CommandRuleError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| CommandRuleError::Persistence(error.to_string()))?;
        }
        let content = toml::to_string_pretty(&CommandRuleFile {
            version: file_version(),
            rules: next.clone(),
        })
        .map_err(|error| CommandRuleError::Persistence(error.to_string()))?;
        let temp = self.path.with_extension(format!("tmp-{}", Uuid::new_v4()));
        std::fs::write(&temp, content)
            .map_err(|error| CommandRuleError::Persistence(error.to_string()))?;
        replace_file(&temp, &self.path).map_err(|error| {
            let _ = std::fs::remove_file(&temp);
            CommandRuleError::Persistence(error.to_string())
        })?;
        *self.rules.write() = next;
        Ok(())
    }
}

fn emit_rule_audit(id: &str, decision: &str, pattern: &[String]) {
    agent_diva_core::audit::emit(agent_diva_core::audit::AuditEvent::DecisionPoint {
        agent_id: id.to_string(),
        decision: decision.to_string(),
        context: format!("pattern={}", pattern.join(" ")),
    });
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::rename(source, target)
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| std::io::Error::other(error.to_string()))
    }
}

fn parse_rule_file(content: &str) -> Result<Vec<CommandRule>, CommandRuleError> {
    if let Ok(file) = toml::from_str::<CommandRuleFile>(content) {
        return Ok(file.rules);
    }
    let legacy: crate::rules::Policy = toml::from_str(content)
        .map_err(|error| CommandRuleError::InvalidFile(error.to_string()))?;
    Ok(legacy
        .prefix_rules
        .into_iter()
        .map(|rule| CommandRule {
            id: stable_legacy_id(&rule.pattern),
            pattern: rule.pattern,
            decision: rule.decision.to_string().to_lowercase(),
            enabled: true,
            source: CommandRuleSource::Legacy,
            justification: rule.justification.unwrap_or_default(),
            created_at: Utc::now(),
            revision: 1,
        })
        .collect())
}

fn stable_legacy_id(pattern: &[String]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in pattern.join("\0").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("legacy-{hash:016x}")
}

pub fn safe_prefix_suggestion(command: &str) -> Option<SafePrefixSuggestion> {
    if command.contains(['|', '>', '<', ';', '&', '\n', '\r']) {
        return None;
    }
    let tokens = shell_words::split(command).ok()?;
    let executable = Path::new(tokens.first()?)
        .file_name()?
        .to_str()?
        .to_ascii_lowercase();
    let args = &tokens[1..];
    let safe = match (executable.as_str(), args) {
        ("git" | "git.exe", [arg]) if matches!(arg.as_str(), "--version" | "status") => true,
        ("git" | "git.exe", [a, b])
            if a == "status" && matches!(b.as_str(), "--short" | "--porcelain" | "-s") =>
        {
            true
        }
        ("git" | "git.exe", [a, b]) if a == "branch" && b == "--show-current" => true,
        ("git" | "git.exe", [a, b])
            if a == "rev-parse"
                && matches!(
                    b.as_str(),
                    "--show-toplevel" | "--show-prefix" | "--is-inside-work-tree" | "HEAD"
                ) =>
        {
            true
        }
        ("git" | "git.exe", [a, b, c])
            if a == "rev-parse" && b == "--abbrev-ref" && c == "HEAD" =>
        {
            true
        }
        ("cargo" | "cargo.exe" | "rustc" | "rustc.exe" | "rustup" | "rustup.exe", [arg])
            if matches!(arg.as_str(), "--version" | "-V") =>
        {
            true
        }
        ("whoami" | "whoami.exe" | "hostname" | "hostname.exe", []) => true,
        _ => false,
    };
    safe.then(|| SafePrefixSuggestion {
        pattern: tokens,
        justification: "Validated read-only command pattern".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestion_matrix_is_conservative() {
        for command in [
            "git status",
            "git status --short",
            "git branch --show-current",
            "git rev-parse --show-toplevel",
            "cargo --version",
            "whoami",
        ] {
            assert!(safe_prefix_suggestion(command).is_some(), "{command}");
        }
        for command in [
            "git reset --hard",
            "cargo build",
            "python -c pass",
            "git status > out",
            "git status && whoami",
            "rm file",
        ] {
            assert!(safe_prefix_suggestion(command).is_none(), "{command}");
        }
    }

    #[test]
    fn persists_updates_and_removals() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("execpolicy.toml");
        let store = CommandRuleStore::open(path.clone()).unwrap();
        let suggestion = safe_prefix_suggestion("git status").unwrap();
        let rule = store.add_suggestion(&suggestion).unwrap();
        assert!(store.allows("git status"));
        let rule = store.set_enabled(&rule.id, rule.revision, false).unwrap();
        assert!(!store.allows("git status"));
        assert!(matches!(
            store.set_enabled(&rule.id, 1, true),
            Err(CommandRuleError::RevisionConflict)
        ));
        store.delete(&rule.id, rule.revision).unwrap();
        assert!(CommandRuleStore::open(path).unwrap().list().is_empty());
    }

    #[test]
    fn invalid_file_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("execpolicy.toml");
        std::fs::write(&path, "not valid = [").unwrap();
        assert!(matches!(
            CommandRuleStore::open(path),
            Err(CommandRuleError::InvalidFile(_))
        ));
    }
}
