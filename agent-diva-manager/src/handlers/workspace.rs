//! Workspace status endpoints.
//!
//! Exposes the active workspace root plus AGENTS.md observability metadata
//! so the desktop GUI and any external operator can display the workspace
//! chip and refresh when the user switches projects.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// GET /api/workspace response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatusResponse {
    /// Resolved workspace root (absolute, canonicalized).
    pub root: String,
    /// `"configured"` when saved in config, `"explicit-cli"` when passed on the
    /// CLI, `"legacy-default"` when the config still matches the historical
    /// `~/.agent-diva/workspace` default, `"process-cwd"` otherwise.
    pub source: String,
    /// Whether this runtime is currently using the persisted default workspace.
    /// An active runtime can become detached when the default is changed while
    /// the gateway stays alive; in that case this is `false` even if the
    /// original resolution source was `configured`.
    pub uses_default_workspace: bool,
    /// Doctor-facing hint when the legacy default is still active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_hint: Option<String>,
    /// AGENTS.md observability when the file exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents_md: Option<AgentsMdStatus>,
}

/// AGENTS.md metadata reported by the workspace endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsMdStatus {
    /// Absolute path to the source file.
    pub path: String,
    /// SHA-256 hex prefix of the trimmed content.
    pub digest: String,
    /// Whether the content was truncated to the prompt budget.
    pub truncated: bool,
    /// Character count of the trimmed content before truncation.
    pub char_count: usize,
    /// Whether the source file is present on disk.
    pub present: bool,
}

/// Handler for `GET /api/workspace`.
///
/// Lightweight (no ManagerCommand roundtrip): reads the workspace root and
/// `AGENTS.md` directly. Suitable for the desktop GUI's workspace chip.
pub async fn get_workspace_handler(State(state): State<AppState>) -> Json<WorkspaceStatusResponse> {
    let root = &state.workspace_context.root;
    let source = state.workspace_context.source.to_string();
    let configured_workspace = configured_workspace_string(&state).await;
    let uses_default_workspace = runtime_uses_current_default(&state, &configured_workspace);

    let legacy_hint = agent_diva_core::workspace::legacy_default_doctor_hint(&configured_workspace);

    let agents_md_path = root.join("AGENTS.md");
    let agents_md = if agents_md_path.exists() {
        agent_diva_agent::workspace_instructions::load_workspace_instructions(root).map(|instr| {
            AgentsMdStatus {
                path: instr.source.display().to_string(),
                digest: instr.digest,
                truncated: instr.truncated,
                char_count: instr.char_count,
                present: true,
            }
        })
    } else {
        None
    };

    Json(WorkspaceStatusResponse {
        root: root.display().to_string(),
        source,
        uses_default_workspace,
        legacy_hint,
        agents_md,
    })
}

fn runtime_uses_current_default(state: &AppState, configured_workspace: &str) -> bool {
    if state.workspace_context.source == agent_diva_core::workspace::WorkspaceSource::ExplicitCli {
        return false;
    }

    let configured_root =
        if configured_workspace == agent_diva_core::workspace::LEGACY_DEFAULT_WORKSPACE {
            // The desktop GUI projects the legacy value to its stable profile-local
            // workspace before bootstrapping the embedded manager. Keep that
            // projection visible to the status endpoint without rewriting config.
            let gui_default = state.config_dir.join("workspace");
            if std::fs::canonicalize(&gui_default).ok().as_deref()
                == std::fs::canonicalize(&state.workspace_context.root)
                    .ok()
                    .as_deref()
            {
                gui_default
            } else {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            }
        } else {
            agent_diva_core::workspace::resolve_workspace(None, configured_workspace).root
        };

    let active_root = std::fs::canonicalize(&state.workspace_context.root)
        .unwrap_or_else(|_| state.workspace_context.root.clone());
    let configured_root = std::fs::canonicalize(&configured_root).unwrap_or(configured_root);
    active_root == configured_root
}

async fn configured_workspace_string(state: &AppState) -> String {
    let config_path = state.config_dir.join("config.json");
    let raw = match std::fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(_) => return agent_diva_core::workspace::LEGACY_DEFAULT_WORKSPACE.to_string(),
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return agent_diva_core::workspace::LEGACY_DEFAULT_WORKSPACE.to_string(),
    };
    value
        .get("agents")
        .and_then(|a| a.get("defaults"))
        .and_then(|d| d.get("workspace"))
        .and_then(|w| w.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| agent_diva_core::workspace::LEGACY_DEFAULT_WORKSPACE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    use crate::state::AppState;

    async fn make_state(workspace_root: &std::path::Path) -> AppState {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel::<crate::ManagerCommand>(4);
        AppState::new(
            api_tx,
            agent_diva_core::bus::AgentEventBus::new(),
            workspace_root,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn returns_root_and_agents_md_metadata_when_present() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("AGENTS.md"),
            "# Project rules\n- rule A\n- rule B\n",
        )
        .unwrap();

        let state = make_state(temp.path()).await;
        let app = Router::new()
            .route("/api/workspace", get(get_workspace_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/workspace")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap();
        let payload: WorkspaceStatusResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            std::fs::canonicalize(payload.root).unwrap(),
            temp.path().canonicalize().unwrap()
        );
        let md = payload.agents_md.expect("AGENTS.md should be reported");
        assert!(md.present);
        assert!(!md.truncated);
        assert!(!md.digest.is_empty());
        assert!(md.char_count > 0);
    }

    #[tokio::test]
    async fn agents_md_absent_omits_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let state = make_state(temp.path()).await;
        let app = Router::new()
            .route("/api/workspace", get(get_workspace_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/workspace")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap();
        let payload: WorkspaceStatusResponse = serde_json::from_slice(&body).unwrap();
        assert!(
            payload.agents_md.is_none(),
            "AGENTS.md missing must leave metadata absent"
        );
    }

    #[tokio::test]
    async fn source_is_projected_from_authoritative_workspace_context() {
        let temp = tempfile::tempdir().unwrap();
        let mut state = make_state(temp.path()).await;
        state.workspace_context.source = agent_diva_core::workspace::WorkspaceSource::ProcessCwd;
        let app = Router::new()
            .route("/api/workspace", get(get_workspace_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/workspace")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap();
        let payload: WorkspaceStatusResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.source, "process-cwd");
    }

    #[tokio::test]
    async fn changed_default_marks_the_active_configured_runtime_as_detached() {
        let config_dir = tempfile::tempdir().unwrap();
        let active_workspace = tempfile::tempdir().unwrap();
        let new_default = tempfile::tempdir().unwrap();
        let loader = agent_diva_core::config::ConfigLoader::with_dir(config_dir.path());
        let mut config = agent_diva_core::config::schema::Config::default();
        config.agents.defaults.workspace = new_default.path().display().to_string();
        loader.save(&config).unwrap();

        let mut state = make_state(active_workspace.path()).await;
        state.config_dir = std::path::PathBuf::from(config_dir.path());
        state.workspace_context.source = agent_diva_core::workspace::WorkspaceSource::Configured;

        let app = Router::new()
            .route("/api/workspace", get(get_workspace_handler))
            .with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/workspace")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap();
        let payload: WorkspaceStatusResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload.source, "configured");
        assert!(!payload.uses_default_workspace);
    }

    #[tokio::test]
    async fn legacy_default_root_emits_doctor_hint() {
        let home = dirs::home_dir().expect("home");
        let legacy = home.join(".agent-diva").join("workspace");
        if !legacy.exists() {
            std::fs::create_dir_all(&legacy).unwrap();
        }
        let mut state = make_state(&legacy).await;
        state.workspace_context.source = agent_diva_core::workspace::WorkspaceSource::LegacyDefault;
        let app = Router::new()
            .route("/api/workspace", get(get_workspace_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/workspace")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap();
        let payload: WorkspaceStatusResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.source, "legacy-default");
        assert!(
            payload.legacy_hint.is_some(),
            "legacy-default root should surface a doctor-facing hint"
        );
    }
}
