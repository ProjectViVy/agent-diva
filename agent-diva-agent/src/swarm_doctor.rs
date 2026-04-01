//! Swarm / capability diagnostics for `config doctor --swarm`, gateway HTTP, and structured status (NFR-R2).

use agent_diva_core::config::schema::Config;
use serde::Serialize;
use std::path::Path;

use crate::capability::{
    parse_and_validate_manifest_from_str, persist::format_manifest_errors,
    persist::workspace_capability_manifest_path, PlaceholderCapabilityRegistry,
};
use crate::subagent_tool_capabilities::{SubagentToolRiskTier, SUBAGENT_TOOL_CAPABILITY_SPECS};

const SWARM_DOCTOR_ID_PREVIEW: usize = 12;

/// **NFR-R2:** For CLI and structured status only — not part of the user chat transcript.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SwarmCortexDoctorV1 {
    /// Bump when adding/removing JSON fields for GUI whitelist contracts (NFR-I2).
    pub schema_version: u32,
    /// Explicit channel tag so consumers never treat this subtree as end-user message content.
    pub channel: &'static str,
    pub capabilities: CapabilityDoctorSummary,
    /// Built-in Layer-A subagent tool catalog (SWARM-MIG-01 / Story 6.5); not merged into workspace manifest ids.
    pub subagent_tools: SubagentToolsDoctorSummary,
    pub cortex: CortexDoctorSummary,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SubagentToolsDoctorSummary {
    pub catalog_version: u32,
    pub entries: Vec<SubagentToolDoctorEntry>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SubagentToolDoctorEntry {
    pub capability_id: String,
    pub tool_name: String,
    pub display_name: String,
    pub description: String,
    pub risk_tier: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub availability_note: String,
}

fn risk_tier_label(t: SubagentToolRiskTier) -> &'static str {
    match t {
        SubagentToolRiskTier::Low => "low",
        SubagentToolRiskTier::Medium => "medium",
        SubagentToolRiskTier::High => "high",
    }
}

fn subagent_tools_doctor_summary() -> SubagentToolsDoctorSummary {
    SubagentToolsDoctorSummary {
        catalog_version: 1,
        entries: SUBAGENT_TOOL_CAPABILITY_SPECS
            .iter()
            .map(|s| SubagentToolDoctorEntry {
                capability_id: s.capability_id.to_string(),
                tool_name: s.tool_name.to_string(),
                display_name: s.display_name.to_string(),
                description: s.description.to_string(),
                risk_tier: risk_tier_label(s.risk_tier).to_string(),
                availability_note: s.availability_note.to_string(),
            })
            .collect(),
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CapabilityDoctorSummary {
    /// `process_registry` when a registry reference was supplied; `workspace_manifest` when loaded from file only; `manifest_file_invalid` when the file fails validation; `unavailable` for standalone CLI without file.
    pub source: String,
    pub count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ids_preview: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CortexDoctorSummary {
    pub state: String,
    pub note: String,
    pub gateway_bind: String,
}

/// Build the versioned swarm/cortex/capability block. Pass `registry` when running inside a process
/// that holds the gateway's [`PlaceholderCapabilityRegistry`]; standalone CLI uses `None` and may pass
/// `workspace_for_manifest_file` for the shared workspace JSON (Story 6.3).
#[must_use]
pub fn swarm_cortex_doctor_section(
    config: &Config,
    registry: Option<&PlaceholderCapabilityRegistry>,
) -> SwarmCortexDoctorV1 {
    let capabilities = if let Some(reg) = registry {
        let s = reg.summary();
        let ids_preview: Vec<String> = s.ids.into_iter().take(SWARM_DOCTOR_ID_PREVIEW).collect();
        CapabilityDoctorSummary {
            source: "process_registry".to_string(),
            count: s.count,
            ids_preview,
            note: String::new(),
        }
    } else {
        CapabilityDoctorSummary {
            source: "unavailable".to_string(),
            count: 0,
            ids_preview: vec![],
            note: "No in-process capability registry in this CLI invocation; live counts require a gateway or embedded registry handle. See architecture.md (capabilities / doctor hook).".to_string(),
        }
    };

    let gateway_bind = format!("{}:{}", config.gateway.host, config.gateway.port);
    let cortex = CortexDoctorSummary {
        state: "n/a".to_string(),
        note: "Cortex on/off is not yet a config field; use gateway process + Epic 1 contracts when wired. This line is CLI/status diagnostics only (NFR-R2).".to_string(),
        gateway_bind,
    };

    SwarmCortexDoctorV1 {
        schema_version: 2,
        channel: "cli_diagnostics",
        capabilities,
        subagent_tools: subagent_tools_doctor_summary(),
        cortex,
    }
}

/// Prefer `process_registry` when set (gateway). Otherwise, if `workspace_for_manifest_file` is set
/// and `.agent-diva/capability-manifest.json` exists, load from disk for real counts or validation notes.
#[must_use]
pub fn swarm_cortex_doctor_section_for_diagnostics(
    config: &Config,
    process_registry: Option<&PlaceholderCapabilityRegistry>,
    workspace_for_manifest_file: Option<&Path>,
) -> SwarmCortexDoctorV1 {
    if let Some(reg) = process_registry {
        return swarm_cortex_doctor_section(config, Some(reg));
    }

    if let Some(ws) = workspace_for_manifest_file {
        let path = workspace_capability_manifest_path(ws);
        if path.exists() {
            return match std::fs::read_to_string(&path) {
                Ok(s) => match parse_and_validate_manifest_from_str(s.trim()) {
                    Ok(m) => {
                        let reg = PlaceholderCapabilityRegistry::new();
                        match reg.replace_with_manifest(m) {
                            Ok(()) => swarm_cortex_doctor_section(config, Some(&reg)),
                            Err(e) => invalid_manifest_section(config, &format_manifest_errors(&e)),
                        }
                    }
                    Err(e) => invalid_manifest_section(config, &format_manifest_errors(&e)),
                },
                Err(e) => invalid_manifest_section(
                    config,
                    &format!("read {}: {e}", path.display()),
                ),
            };
        }
    }

    swarm_cortex_doctor_section(config, None)
}

/// Gateway manager path: use in-process registry counts unless bootstrap failed to load the
/// workspace file and the registry is still empty — then surface `manifest_file_invalid` (Story 6.3).
#[must_use]
pub fn swarm_cortex_doctor_for_gateway(
    config: &Config,
    registry: &PlaceholderCapabilityRegistry,
    bootstrap_manifest_error: Option<&str>,
) -> SwarmCortexDoctorV1 {
    let summary = registry.summary();
    if summary.count == 0 {
        if let Some(note) = bootstrap_manifest_error.filter(|s| !s.is_empty()) {
            return invalid_manifest_section(config, note);
        }
    }
    swarm_cortex_doctor_section_for_diagnostics(config, Some(registry), None)
}

fn invalid_manifest_section(config: &Config, note: &str) -> SwarmCortexDoctorV1 {
    let gateway_bind = format!("{}:{}", config.gateway.host, config.gateway.port);
    SwarmCortexDoctorV1 {
        schema_version: 2,
        channel: "cli_diagnostics",
        capabilities: CapabilityDoctorSummary {
            source: "manifest_file_invalid".to_string(),
            count: 0,
            ids_preview: vec![],
            note: note.to_string(),
        },
        subagent_tools: subagent_tools_doctor_summary(),
        cortex: CortexDoctorSummary {
            state: "n/a".to_string(),
            note: "Cortex on/off is not yet a config field; use gateway process + Epic 1 contracts when wired. This line is CLI/status diagnostics only (NFR-R2).".to_string(),
            gateway_bind,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::config::schema::Config;

    #[test]
    fn swarm_section_unavailable_without_registry() {
        let cfg = Config::default();
        let s = swarm_cortex_doctor_section(&cfg, None);
        assert_eq!(s.schema_version, 2);
        assert_eq!(s.channel, "cli_diagnostics");
        assert_eq!(s.capabilities.source, "unavailable");
        assert_eq!(s.capabilities.count, 0);
        assert!(!s.capabilities.note.is_empty());
        assert_eq!(s.cortex.state, "n/a");
        assert!(s.cortex.gateway_bind.contains(':'));
        assert_eq!(s.subagent_tools.entries.len(), 6);
        assert!(
            s.subagent_tools
                .entries
                .iter()
                .any(|e| e.capability_id == "tool.subagent.exec")
        );
    }

    #[test]
    fn swarm_section_uses_registry_when_provided() {
        let cfg = Config::default();
        let reg = PlaceholderCapabilityRegistry::new();
        let m = parse_and_validate_manifest_from_str(
            r#"{"schema_version":"0","capabilities":[{"id":"cap-a","name":"A"}]}"#,
        )
        .unwrap();
        reg.register(m).unwrap();
        let s = swarm_cortex_doctor_section(&cfg, Some(&reg));
        assert_eq!(s.capabilities.source, "process_registry");
        assert_eq!(s.capabilities.count, 1);
        assert_eq!(s.capabilities.ids_preview, vec!["cap-a".to_string()]);
        assert!(s.capabilities.note.is_empty());
    }

    #[test]
    fn gateway_doctor_surfaces_bootstrap_error_when_registry_empty() {
        let cfg = Config::default();
        let reg = PlaceholderCapabilityRegistry::new();
        let s = swarm_cortex_doctor_for_gateway(&cfg, &reg, Some("invalid manifest"));
        assert_eq!(s.capabilities.source, "manifest_file_invalid");
        assert!(s.capabilities.note.contains("invalid manifest"));
        assert_eq!(s.capabilities.count, 0);
    }

    #[test]
    fn gateway_doctor_ignores_stale_bootstrap_error_when_registry_populated() {
        let cfg = Config::default();
        let reg = PlaceholderCapabilityRegistry::new();
        let m = parse_and_validate_manifest_from_str(
            r#"{"schema_version":"0","capabilities":[{"id":"c","name":"C"}]}"#,
        )
        .unwrap();
        reg.register(m).unwrap();
        let s = swarm_cortex_doctor_for_gateway(&cfg, &reg, Some("stale bootstrap message"));
        assert_eq!(s.capabilities.source, "process_registry");
        assert_eq!(s.capabilities.count, 1);
    }

    #[test]
    fn diagnostics_loads_workspace_manifest_when_no_process_registry() {
        let tmp = tempfile::tempdir().unwrap();
        let agent_dir = tmp.path().join(".agent-diva");
        std::fs::create_dir_all(&agent_dir).unwrap();
        let manifest_path = agent_dir.join("capability-manifest.json");
        std::fs::write(
            &manifest_path,
            r#"{"schema_version":"0","capabilities":[{"id":"w-cap","name":"W"}]}"#,
        )
        .unwrap();
        let cfg = Config::default();
        let s = swarm_cortex_doctor_section_for_diagnostics(&cfg, None, Some(tmp.path()));
        assert_eq!(s.capabilities.source, "process_registry");
        assert_eq!(s.capabilities.count, 1);
        assert_eq!(s.capabilities.ids_preview, vec!["w-cap".to_string()]);
    }
}
