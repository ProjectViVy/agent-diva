//! Shared audit log line parser.
//!
//! Reconciles the two former copies in `agent-diva-gui` and `agent-diva-manager`
//! into a single canonical implementation. Both consumers now delegate here.
//!
//! # Behavioral reconciliation
//!
//! Four differences existed between the GUI and manager copies:
//!
//! 1. **Fallback for unparseable events**: Manager returned `Some("unknown")`,
//!    GUI returned `None`. Reconciled: return `Some("unknown")` — preserves
//!    all parseable log lines for debugging.
//! 2. **Message-fallback data key**: Manager used `{"message":…}`, GUI used
//!    `{"raw":…}`. Reconciled: `{"raw":…}` — consistent with the structured
//!    `event` field and clearer about being unparsed output.
//! 3. **Missing `data` in direct format**: Manager defaulted to `{}`, GUI
//!    defaulted to `Null`. Reconciled: `Null` — more honest about absence.
//! 4. **`rust_variant_to_snake_case` robustness**: Manager did simple
//!    uppercase→lowercase; GUI also stripped Debug-format struct fields
//!    (`ToolInvoked { … }`) and filtered non-alphanumeric chars.
//!    Reconciled: GUI's version — handles real Debug output.

use serde::{Deserialize, Serialize};

/// Parsed audit event DTO shared by GUI and manager.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEventDto {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

/// Parse a single JSON log line into an `AuditEventDto`.
///
/// Supports two log formats:
/// 1. **Structured** (tracing_subscriber JSON): `{"timestamp":"…","fields":{"event":"…"}}`
/// 2. **Direct** (serde tagged enum): `{"timestamp":"…","type":"tool_invoked","data":{…}}`
///
/// Returns `None` only if the line is not valid JSON or lacks a `timestamp`
/// string field. Otherwise always returns `Some` — unknown events get
/// `event_type: "unknown"`.
pub fn parse_audit_event_from_json_line(line: &str) -> Option<AuditEventDto> {
    let parsed: serde_json::Value = serde_json::from_str(line).ok()?;
    let timestamp = parsed.get("timestamp")?.as_str()?.to_string();

    // Try structured format (tracing_subscriber JSON output)
    if let Some(fields) = parsed.get("fields") {
        if let Some(event_val) = fields.get("event") {
            if let Some(event_str) = event_val.as_str() {
                let event_type = rust_variant_to_snake_case(event_str);
                return Some(AuditEventDto {
                    event_type,
                    data: serde_json::json!({"raw": event_str}),
                    timestamp,
                });
            }
        }
        // Try message field as fallback
        if let Some(msg_val) = fields.get("message") {
            if let Some(msg_str) = msg_val.as_str() {
                let event_type = rust_variant_to_snake_case(msg_str);
                return Some(AuditEventDto {
                    event_type,
                    data: serde_json::json!({"raw": msg_str}),
                    timestamp,
                });
            }
        }
    }

    // Try direct format: {"timestamp":"...","type":"tool_invoked","data":{...}}
    if let Some(type_val) = parsed.get("type").and_then(|v| v.as_str()) {
        let data = parsed
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        return Some(AuditEventDto {
            event_type: type_val.to_string(),
            data,
            timestamp,
        });
    }

    // Fallback: return the whole line as a generic event
    Some(AuditEventDto {
        event_type: "unknown".to_string(),
        data: serde_json::json!({"raw": line}),
        timestamp,
    })
}

/// Convert Rust-style PascalCase variant names to snake_case.
///
/// Handles Debug-format strings like `ToolInvoked { tool_name: "…" }`
/// by stripping content after `{` or `(`, and filtering non-alphanumeric
/// characters (except underscore).
pub fn rust_variant_to_snake_case(s: &str) -> String {
    let name = s.split(&['{', '('][..]).next().unwrap_or(s);
    let mut result = String::new();
    for (i, ch) in name.char_indices() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        if ch.is_alphanumeric() || ch == '_' {
            result.push(ch.to_ascii_lowercase());
        }
    }
    if result.is_empty() {
        result = "unknown".to_string();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── rust_variant_to_snake_case ──────────────────────────────────

    #[test]
    fn snake_case_simple_variant() {
        assert_eq!(rust_variant_to_snake_case("ToolInvoked"), "tool_invoked");
    }

    #[test]
    fn snake_case_multi_word() {
        assert_eq!(
            rust_variant_to_snake_case("SessionCreated"),
            "session_created"
        );
    }

    #[test]
    fn snake_case_single_word() {
        assert_eq!(rust_variant_to_snake_case("Error"), "error");
    }

    #[test]
    fn snake_case_already_lowercase() {
        assert_eq!(rust_variant_to_snake_case("lowercase"), "lowercase");
    }

    #[test]
    fn snake_case_strips_debug_fields() {
        assert_eq!(
            rust_variant_to_snake_case("ToolInvoked { tool_name: \"bash\" }"),
            "tool_invoked"
        );
    }

    #[test]
    fn snake_case_strips_tuple_fields() {
        assert_eq!(
            rust_variant_to_snake_case("HeartbeatTriggered(5)"),
            "heartbeat_triggered"
        );
    }

    #[test]
    fn snake_case_empty_becomes_unknown() {
        assert_eq!(rust_variant_to_snake_case("!!!"), "unknown");
    }

    // ── parse_audit_event_from_json_line: structured format ─────────

    #[test]
    fn parse_structured_tracing_json() {
        let line = r#"{"timestamp":"2026-07-01T12:00:00Z","level":"INFO","target":"audit","fields":{"event":"ToolInvoked","tool":"shell"}}"#;
        let evt = parse_audit_event_from_json_line(line).unwrap();
        assert_eq!(evt.event_type, "tool_invoked");
        assert_eq!(evt.timestamp, "2026-07-01T12:00:00Z");
        assert_eq!(evt.data["raw"], "ToolInvoked");
    }

    #[test]
    fn parse_structured_message_fallback() {
        let line = r#"{"timestamp":"2026-07-01T12:00:00Z","fields":{"message":"ToolInvoked"}}"#;
        let evt = parse_audit_event_from_json_line(line).unwrap();
        assert_eq!(evt.event_type, "tool_invoked");
        assert_eq!(evt.data["raw"], "ToolInvoked");
    }

    // ── parse_audit_event_from_json_line: direct format ─────────────

    #[test]
    fn parse_direct_format_with_data() {
        let line =
            r#"{"timestamp":"2026-07-01T12:00:00Z","type":"tool_invoked","data":{"tool":"shell"}}"#;
        let evt = parse_audit_event_from_json_line(line).unwrap();
        assert_eq!(evt.event_type, "tool_invoked");
        assert_eq!(evt.data["tool"], "shell");
    }

    #[test]
    fn parse_direct_format_missing_data_yields_null() {
        let line = r#"{"timestamp":"2026-07-01T12:00:00Z","type":"heartbeat_triggered"}"#;
        let evt = parse_audit_event_from_json_line(line).unwrap();
        assert_eq!(evt.event_type, "heartbeat_triggered");
        assert!(evt.data.is_null(), "missing data should be Null, not {{}}");
    }

    // ── parse_audit_event_from_json_line: unknown / fallback ────────

    #[test]
    fn parse_unknown_json_returns_unknown_event_type() {
        let line =
            r#"{"timestamp":"2026-07-01T12:00:00Z","level":"INFO","message":"something else"}"#;
        let evt = parse_audit_event_from_json_line(line).unwrap();
        assert_eq!(evt.event_type, "unknown");
        assert_eq!(evt.data["raw"], line);
    }

    // ── parse_audit_event_from_json_line: error cases ───────────────

    #[test]
    fn parse_non_json_returns_none() {
        assert!(parse_audit_event_from_json_line("not json").is_none());
    }

    #[test]
    fn parse_json_without_timestamp_returns_none() {
        let line = r#"{"type":"tool_invoked","data":{}}"#;
        assert!(parse_audit_event_from_json_line(line).is_none());
    }

    #[test]
    fn parse_json_with_null_timestamp_returns_none() {
        let line = r#"{"timestamp":null,"type":"tool_invoked"}"#;
        assert!(parse_audit_event_from_json_line(line).is_none());
    }

    #[test]
    fn parse_empty_string_returns_none() {
        assert!(parse_audit_event_from_json_line("").is_none());
    }

    // ── parse_audit_event_from_json_line: known AuditEvent variants ─

    #[test]
    fn parse_all_known_variants_direct_format() {
        let variants = [
            ("heartbeat_triggered", r#"{"interval_secs":5}"#),
            ("tool_invoked", r#"{"tool_name":"bash","args":{}}"#),
            ("tool_denied", r#"{"tool_name":"rm","reason":"policy"}"#),
            (
                "decision_point",
                r#"{"agent_id":"a1","decision":"go","context":"loop"}"#,
            ),
            (
                "injection_detected",
                r#"{"layer":"input","pattern":"role_override","severity":"high"}"#,
            ),
            (
                "pii_redacted",
                r#"{"category":"email","severity":"warning","count":3}"#,
            ),
            (
                "token_used",
                r#"{"provider":"openai","model":"gpt-4","tokens":150}"#,
            ),
            (
                "reasoning_received",
                r#"{"content_len":500,"model":"claude"}"#,
            ),
            ("chat_over", r#"{"session_id":"s1","turn_count":42}"#),
            ("presence_changed", r#"{"from":"active","to":"gone"}"#),
        ];
        for (event_type, data_json) in &variants {
            let line = format!(
                r#"{{"timestamp":"2026-07-01T12:00:00Z","type":"{}","data":{}}}"#,
                event_type, data_json
            );
            let evt = parse_audit_event_from_json_line(&line).unwrap();
            assert_eq!(
                evt.event_type, *event_type,
                "failed for variant {}",
                event_type
            );
        }
    }
}
