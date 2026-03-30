//! v0 capability manifest: JSON parse + explicit validation (serde-friendly output types).

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{codes, CapabilityManifestError, CapabilityManifestErrors};

/// Validated v0 manifest entry (in-memory contract).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedCapability {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

/// Validated root document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedManifest {
    /// Optional; v0 accepts `"0"` or omission. Other values yield a field error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    pub capabilities: Vec<ValidatedCapability>,
}

/// v0 required/optional keys (frozen):
///
/// | Key | Required | Type | Notes |
/// |-----|----------|------|-------|
/// | `capabilities` | yes | array of objects | Each object must include `id` |
/// | `schema_version` | no | string | If present, must be `"0"` for this validator |
///
/// Per-entry:
///
/// | Key | Required | Type | Notes |
/// |-----|----------|------|-------|
/// | `id` | yes | non-empty string | Unique across `capabilities` |
/// | `name` | no | string | |
/// | `description` | no | string | |
/// | `priority` | no | integer | If present, must be in **0..=1000** (v0 band) |
const PRIORITY_MIN: i64 = 0;
const PRIORITY_MAX: i64 = 1000;

fn err_vec(e: CapabilityManifestError) -> CapabilityManifestErrors {
    CapabilityManifestErrors::new(vec![e])
}

/// Parse UTF-8 bytes as JSON, then validate v0 rules. Returns **all** field-level issues where possible.
pub fn parse_and_validate_manifest_from_bytes(bytes: &[u8]) -> Result<ValidatedManifest, CapabilityManifestErrors> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        err_vec(CapabilityManifestError::file(
            codes::INVALID_UTF8,
            "manifest is not valid UTF-8",
        ))
    })?;
    parse_and_validate_manifest_from_str(text)
}

/// Parse JSON string, then validate v0 rules.
pub fn parse_and_validate_manifest_from_str(text: &str) -> Result<ValidatedManifest, CapabilityManifestErrors> {
    let value: Value = serde_json::from_str(text).map_err(|e| {
        err_vec(CapabilityManifestError::file(
            codes::JSON_PARSE,
            format!("invalid JSON: {e}"),
        ))
    })?;
    validate_v0_value(&value)
}

/// Read a file and validate (I/O errors are **file**-scoped).
pub fn parse_and_validate_manifest_from_path(path: &Path) -> Result<ValidatedManifest, CapabilityManifestErrors> {
    let bytes = fs::read(path).map_err(|e| {
        err_vec(CapabilityManifestError::file(
            codes::IO_READ,
            format!("failed to read {}: {e}", path.display()),
        ))
    })?;
    parse_and_validate_manifest_from_bytes(&bytes)
}

fn validate_v0_value(value: &Value) -> Result<ValidatedManifest, CapabilityManifestErrors> {
    let mut errors = CapabilityManifestErrors::default();
    let Some(root) = value.as_object() else {
        errors.push(CapabilityManifestError::field(
            codes::NOT_OBJECT,
            "manifest root must be a JSON object",
            "/",
        ));
        return Err(errors);
    };

    let mut schema_version: Option<String> = None;
    if let Some(sv) = root.get("schema_version") {
        if let Some(s) = sv.as_str() {
            if s != "0" {
                errors.push(CapabilityManifestError::field(
                    codes::SCHEMA_VERSION_UNSUPPORTED,
                    format!("schema_version must be \"0\" for v0 validator, got {s:?}"),
                    "/schema_version",
                ));
            } else {
                schema_version = Some(s.to_string());
            }
        } else {
            errors.push(CapabilityManifestError::field(
                codes::TYPE_MISMATCH,
                "schema_version must be a string",
                "/schema_version",
            ));
        }
    }

    let Some(cap_val) = root.get("capabilities") else {
        errors.push(CapabilityManifestError::field(
            codes::MISSING_FIELD,
            "missing required field \"capabilities\"",
            "/capabilities",
        ));
        return Err(errors);
    };

    let Some(cap_array) = cap_val.as_array() else {
        errors.push(CapabilityManifestError::field(
            codes::TYPE_MISMATCH,
            "\"capabilities\" must be an array",
            "/capabilities",
        ));
        return Err(errors);
    };

    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<ValidatedCapability> = Vec::with_capacity(cap_array.len());

    for (i, item) in cap_array.iter().enumerate() {
        let base = format!("/capabilities/{i}");
        let Some(obj) = item.as_object() else {
            errors.push(CapabilityManifestError::field(
                codes::TYPE_MISMATCH,
                "each capability must be a JSON object",
                &base,
            ));
            continue;
        };

        let id_path = format!("{base}/id");
        let id_val = obj.get("id");
        if id_val.is_none() {
            errors.push(CapabilityManifestError::field(
                codes::MISSING_FIELD,
                "missing required field \"id\"",
                &id_path,
            ));
            continue;
        }
        let Some(id_str) = id_val.and_then(|v| v.as_str()) else {
            errors.push(CapabilityManifestError::field(
                codes::TYPE_MISMATCH,
                "\"id\" must be a non-empty string",
                &id_path,
            ));
            continue;
        };
        if id_str.is_empty() {
            errors.push(CapabilityManifestError::field(
                codes::EMPTY_ID,
                "\"id\" must not be empty",
                &id_path,
            ));
            continue;
        }
        if !seen.insert(id_str.to_string()) {
            errors.push(CapabilityManifestError::field(
                codes::DUPLICATE_ID,
                format!("duplicate capability id {:?}", id_str),
                &id_path,
            ));
            continue;
        }

        let mut name: Option<String> = None;
        if let Some(n) = obj.get("name") {
            if let Some(ns) = n.as_str() {
                name = Some(ns.to_string());
            } else {
                errors.push(CapabilityManifestError::field(
                    codes::TYPE_MISMATCH,
                    "\"name\" must be a string",
                    format!("{base}/name"),
                ));
            }
        }

        let mut description: Option<String> = None;
        if let Some(d) = obj.get("description") {
            if let Some(ds) = d.as_str() {
                description = Some(ds.to_string());
            } else {
                errors.push(CapabilityManifestError::field(
                    codes::TYPE_MISMATCH,
                    "\"description\" must be a string",
                    format!("{base}/description"),
                ));
            }
        }

        let mut priority: Option<i32> = None;
        if let Some(p) = obj.get("priority") {
            if let Some(n) = p.as_i64() {
                if (PRIORITY_MIN..=PRIORITY_MAX).contains(&n) {
                    priority = Some(n as i32);
                } else {
                    errors.push(CapabilityManifestError::field(
                        codes::PRIORITY_OUT_OF_RANGE,
                        format!("priority must be between {PRIORITY_MIN} and {PRIORITY_MAX} inclusive"),
                        format!("{base}/priority"),
                    ));
                }
            } else {
                errors.push(CapabilityManifestError::field(
                    codes::TYPE_MISMATCH,
                    "\"priority\" must be an integer",
                    format!("{base}/priority"),
                ));
            }
        }

        // Unknown keys are ignored in v0 (forward compatibility).

        out.push(ValidatedCapability {
            id: id_str.to_string(),
            name,
            description,
            priority,
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ValidatedManifest {
        schema_version,
        capabilities: out,
    })
}

#[cfg(test)]
mod tests {
    use super::super::error::CapabilityErrorLocationKind;
    use super::*;

    #[test]
    fn from_path_reads_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("capabilities.json");
        std::fs::write(&path, r#"{"capabilities":[{"id":"from_file"}]}"#).unwrap();
        let m = parse_and_validate_manifest_from_path(&path).unwrap();
        assert_eq!(m.capabilities[0].id, "from_file");
    }

    #[test]
    fn valid_minimal_passes() {
        let j = r#"{"capabilities":[{"id":"a"}]}"#;
        let m = parse_and_validate_manifest_from_str(j).unwrap();
        assert_eq!(m.capabilities.len(), 1);
        assert_eq!(m.capabilities[0].id, "a");
        assert!(m.schema_version.is_none());
    }

    #[test]
    fn valid_with_schema_version_zero() {
        let j = r#"{"schema_version":"0","capabilities":[{"id":"x","name":"X","priority":10}]}"#;
        let m = parse_and_validate_manifest_from_str(j).unwrap();
        assert_eq!(m.schema_version.as_deref(), Some("0"));
        assert_eq!(m.capabilities[0].priority, Some(10));
    }

    #[test]
    fn json_syntax_error_is_file_scoped() {
        let err = parse_and_validate_manifest_from_str("{").unwrap_err();
        assert_eq!(err.errors.len(), 1);
        assert_eq!(err.errors[0].code, codes::JSON_PARSE);
        assert_eq!(err.errors[0].location_kind, CapabilityErrorLocationKind::File);
    }

    #[test]
    fn invalid_utf8_is_distinct_from_json_parse() {
        let err = parse_and_validate_manifest_from_bytes(&[0xff, 0xfe, 0xfd]).unwrap_err();
        assert_eq!(err.errors.len(), 1);
        assert_eq!(err.errors[0].code, codes::INVALID_UTF8);
        assert_eq!(err.errors[0].location_kind, CapabilityErrorLocationKind::File);
    }

    #[test]
    fn missing_capabilities() {
        let err = parse_and_validate_manifest_from_str(r#"{"schema_version":"0"}"#).unwrap_err();
        assert!(err.errors.iter().any(|e| e.code == codes::MISSING_FIELD));
    }

    #[test]
    fn duplicate_and_empty_id() {
        let j = r#"{"capabilities":[{"id":"a"},{"id":""},{"id":"a"}]}"#;
        let err = parse_and_validate_manifest_from_str(j).unwrap_err();
        assert!(err.errors.iter().any(|e| e.code == codes::EMPTY_ID));
        assert!(err.errors.iter().any(|e| e.code == codes::DUPLICATE_ID));
    }

    #[test]
    fn priority_out_of_range_and_type_errors() {
        let j = r#"{"capabilities":[{"id":"a","priority":10001},{"id":"b","name":1}]}"#;
        let err = parse_and_validate_manifest_from_str(j).unwrap_err();
        assert!(err.errors.iter().any(|e| e.code == codes::PRIORITY_OUT_OF_RANGE));
        assert!(err.errors.iter().any(|e| e.code == codes::TYPE_MISMATCH));
    }

    #[test]
    fn capabilities_not_array() {
        let err = parse_and_validate_manifest_from_str(r#"{"capabilities":{}}"#).unwrap_err();
        assert!(err.errors.iter().any(|e| e.code == codes::TYPE_MISMATCH));
    }

    #[test]
    fn schema_version_wrong_string() {
        let err =
            parse_and_validate_manifest_from_str(r#"{"schema_version":"2","capabilities":[{"id":"a"}]}"#).unwrap_err();
        assert!(err
            .errors
            .iter()
            .any(|e| e.code == codes::SCHEMA_VERSION_UNSUPPORTED));
    }

    #[test]
    fn errors_serialize_for_dto() {
        let err = parse_and_validate_manifest_from_str("not json").unwrap_err();
        let json = serde_json::to_value(&err).unwrap();
        assert!(json.get("errors").is_some());
    }
}
