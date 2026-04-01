//! Tauri commands for v0 capability manifest (FR10/FR11) — delegates to `agent-diva-agent`.

use agent_diva_agent::capability::{
    parse_and_validate_manifest_from_str, persist_capability_manifest_json, CapabilityManifestError,
    PlaceholderCapabilityRegistry, RegistrySummary,
};
use agent_diva_cli::cli_runtime::CliRuntime;
use agent_diva_core::config::ConfigLoader;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

fn expand_user_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn gui_config_loader() -> ConfigLoader {
    match std::env::var("AGENT_DIVA_CONFIG_DIR") {
        Ok(path) if !path.trim().is_empty() => ConfigLoader::with_dir(expand_user_path(&path)),
        _ => ConfigLoader::new(),
    }
}

/// Always returns HTTP-200 style payload for `invoke` so the frontend can read structured errors without string parsing.
#[derive(Debug, Clone, Serialize)]
pub struct CapabilityManifestSubmitResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<RegistrySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<CapabilityManifestError>>,
    /// Load/persist failures (no structured manifest errors).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Parse and validate JSON, then **replace** the in-process placeholder registry on success via a single write lock (settings editor is source of truth).
#[tauri::command]
pub fn submit_capability_manifest_json(
    registry: State<'_, Arc<PlaceholderCapabilityRegistry>>,
    json: String,
) -> CapabilityManifestSubmitResult {
    match parse_and_validate_manifest_from_str(json.trim()) {
        Ok(manifest) => {
            let loader = gui_config_loader();
            let runtime = CliRuntime::from_paths(
                Some(loader.config_path().to_path_buf()),
                Some(loader.config_dir().to_path_buf()),
                None,
            );
            let persist_result = loader.load().map(|config| {
                let ws = runtime.effective_workspace(&config);
                persist_capability_manifest_json(&ws, json.trim())
            });
            match persist_result {
                Ok(Ok(())) => match registry.replace_with_manifest(manifest) {
                    Ok(()) => CapabilityManifestSubmitResult {
                        ok: true,
                        summary: Some(registry.summary()),
                        errors: None,
                        message: None,
                    },
                    Err(e) => CapabilityManifestSubmitResult {
                        ok: false,
                        summary: None,
                        errors: Some(e.errors),
                        message: None,
                    },
                },
                Ok(Err(e)) => CapabilityManifestSubmitResult {
                    ok: false,
                    summary: None,
                    errors: None,
                    message: Some(format!(
                        "failed to persist capability manifest to workspace: {e}"
                    )),
                },
                Err(e) => CapabilityManifestSubmitResult {
                    ok: false,
                    summary: None,
                    errors: None,
                    message: Some(format!("failed to load config for workspace path: {e}")),
                },
            }
        }
        Err(e) => CapabilityManifestSubmitResult {
            ok: false,
            summary: None,
            errors: Some(e.errors),
            message: None,
        },
    }
}

#[tauri::command]
pub fn get_capability_registry_summary(
    registry: State<'_, Arc<PlaceholderCapabilityRegistry>>,
) -> RegistrySummary {
    registry.summary()
}

