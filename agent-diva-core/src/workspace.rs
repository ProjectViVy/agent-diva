//! Workspace resolution: canonical root, source tracking, and legacy migration.
//!
//! The workspace root is the project execution root where tools, shell, plans,
//! sessions, and AGENTS.md discovery all operate. It is distinct from the
//! machine-level config home (`~/.agent-diva`).
//!
//! Resolution priority:
//! 1. CLI `--workspace DIR` override (always wins).
//! 2. Config `agents.defaults.workspace` — if it equals the legacy default
//!    `~/.agent-diva/workspace`, treat as "not configured" and fall through to
//!    process CWD with a one-time migration warning.
//! 3. Process startup CWD.
//!
//! All resolved paths are immediately canonicalized/absolutized where
//! possible. Global `std::env::set_current_dir` is never called; concurrent
//! Gateway/Channel/Cron runtimes each carry their own [`WorkspaceContext`].

use std::path::{Path, PathBuf};

/// How the workspace root was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSource {
    /// Explicit `--workspace DIR` passed on the CLI.
    ExplicitCli,
    /// User explicitly saved a workspace value in `config.json`
    /// (any value other than the legacy default).
    Configured,
    /// No override and no saved config; resolved to process startup CWD.
    ProcessCwd,
    /// Config value still equals the legacy `~/.agent-diva/workspace` default.
    /// Treated as "not configured"; resolved to process CWD with a migration
    /// warning logged at `warn` level.
    LegacyDefault,
}

impl std::fmt::Display for WorkspaceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExplicitCli => write!(f, "explicit-cli"),
            Self::Configured => write!(f, "configured"),
            Self::ProcessCwd => write!(f, "process-cwd"),
            Self::LegacyDefault => write!(f, "legacy-default"),
        }
    }
}

/// AGENTS.md discovery metadata attached to a [`WorkspaceContext`].
///
/// Populated by the agent crate after reading the workspace root file.
/// The `digest` is a SHA-256 hex prefix of the file content; `truncated` is
/// `true` when the content exceeded the character budget.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentsMdMeta {
    pub path: PathBuf,
    pub digest: String,
    pub truncated: bool,
    pub char_count: usize,
}

/// Resolved workspace context for a single runtime session.
///
/// `root` is the canonical project execution root. `config_dir` (machine-level
/// authority for BML/Persona/config) lives separately on [`CliRuntime`] or the
/// gateway config loader.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceContext {
    pub root: PathBuf,
    pub source: WorkspaceSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents_md: Option<AgentsMdMeta>,
}

/// Legacy default workspace value in the config schema.
pub const LEGACY_DEFAULT_WORKSPACE: &str = "~/.agent-diva/workspace";

/// Resolve the workspace root for the current runtime session.
///
/// `cli_override` wins unconditionally. When the config value equals the legacy
/// default, the process CWD is used and a `warn` log is emitted so the user
/// can pin an explicit workspace via `agent-diva config set --workspace <dir>`.
///
/// All resolved paths are best-effort canonicalized: if the directory exists,
/// `std::fs::canonicalize` resolves symlinks and trailing separators; if it
/// does not exist yet, the absolute path is returned as-is (no silent creation).
pub fn resolve_workspace(cli_override: Option<&Path>, config_workspace: &str) -> WorkspaceContext {
    let is_legacy = config_workspace == LEGACY_DEFAULT_WORKSPACE;

    let (source, raw) = if let Some(p) = cli_override {
        (WorkspaceSource::ExplicitCli, absolutize(p))
    } else if is_legacy {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        (WorkspaceSource::LegacyDefault, cwd)
    } else {
        (WorkspaceSource::Configured, expand_tilde(config_workspace))
    };

    let root = canonicalize_or_keep(&raw);

    if source == WorkspaceSource::LegacyDefault {
        tracing::warn!(
            legacy_default = LEGACY_DEFAULT_WORKSPACE,
            resolved = %root.display(),
            "workspace config still uses legacy default; using process CWD. \
             Run `agent-diva config set --workspace <dir>` to pin a project workspace."
        );
    }

    WorkspaceContext {
        root,
        source,
        agents_md: None,
    }
}

/// Produce a doctor-facing hint for the legacy-default migration.
pub fn legacy_default_doctor_hint(config_workspace: &str) -> Option<String> {
    if config_workspace == LEGACY_DEFAULT_WORKSPACE {
        Some(format!(
            "Workspace default is the legacy value `{}`. \
             Diva now falls back to the process CWD; pin an explicit workspace \
             with `agent-diva config set --workspace <dir>`.",
            LEGACY_DEFAULT_WORKSPACE
        ))
    } else {
        None
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn canonicalize_or_keep(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_default_resolves_to_process_cwd() {
        let ctx = resolve_workspace(None, LEGACY_DEFAULT_WORKSPACE);
        assert_eq!(ctx.source, WorkspaceSource::LegacyDefault);
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(ctx.root, canonicalize_or_keep(&cwd));
    }

    #[test]
    fn cli_override_wins_and_is_absolutized() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("project");
        std::fs::create_dir_all(&target).unwrap();
        let ctx = resolve_workspace(Some(target.as_path()), LEGACY_DEFAULT_WORKSPACE);
        assert_eq!(ctx.source, WorkspaceSource::ExplicitCli);
        assert_eq!(ctx.root, target.canonicalize().unwrap_or(target));
    }

    #[test]
    fn configured_workspace_is_expanded_and_canonicalized() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("my-ws");
        std::fs::create_dir_all(&target).unwrap();
        let ctx = resolve_workspace(None, target.to_str().unwrap());
        assert_eq!(ctx.source, WorkspaceSource::Configured);
        assert_eq!(ctx.root, target.canonicalize().unwrap());
    }

    #[test]
    fn configured_tilde_workspace_expands_to_home() {
        let home = dirs::home_dir().expect("home dir");
        // Use a path that is unlikely to exist so canonicalize falls back to raw.
        let ctx = resolve_workspace(None, "~/agent-diva-test-nonexistent-ws");
        assert_eq!(ctx.source, WorkspaceSource::Configured);
        assert_eq!(ctx.root, home.join("agent-diva-test-nonexistent-ws"));
    }

    #[test]
    fn doctor_hint_only_for_legacy_default() {
        assert!(legacy_default_doctor_hint(LEGACY_DEFAULT_WORKSPACE).is_some());
        assert!(legacy_default_doctor_hint("/some/project").is_none());
    }

    #[test]
    fn relative_cli_override_is_absolutized() {
        let cwd = std::env::current_dir().unwrap();
        let ctx = resolve_workspace(Some(Path::new(".")), "/tmp/something");
        assert_eq!(ctx.source, WorkspaceSource::ExplicitCli);
        let expected = canonicalize_or_keep(&cwd);
        assert_eq!(ctx.root, expected);
    }
}
