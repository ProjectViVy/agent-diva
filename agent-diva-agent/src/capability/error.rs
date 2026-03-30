//! Machine-readable capability manifest errors (FR11 / Epic 4.2 DTO boundary).

use serde::{Deserialize, Serialize};

/// Stable error codes for Tauri/UI logic (do not rename without a version bump).
pub mod codes {
    pub const JSON_PARSE: &str = "capability.manifest.json_parse";
    pub const INVALID_UTF8: &str = "capability.manifest.invalid_utf8";
    pub const IO_READ: &str = "capability.manifest.io_read";
    pub const NOT_OBJECT: &str = "capability.manifest.root_not_object";
    pub const MISSING_FIELD: &str = "capability.manifest.missing_field";
    pub const TYPE_MISMATCH: &str = "capability.manifest.type_mismatch";
    pub const EMPTY_ID: &str = "capability.manifest.empty_id";
    pub const DUPLICATE_ID: &str = "capability.manifest.duplicate_id";
    pub const PRIORITY_OUT_OF_RANGE: &str = "capability.manifest.priority_out_of_range";
    pub const SCHEMA_VERSION_UNSUPPORTED: &str = "capability.manifest.schema_version_unsupported";
    pub const REGISTRY_LOCK_POISONED: &str = "capability.registry.lock_poisoned";
}

/// File-level vs field-level location (NFR-I2 / whitelisted DTO evolution).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityErrorLocationKind {
    File,
    Field,
}

/// One validation or parse issue; serializable for `invoke` passthrough.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityManifestError {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    pub message: String,
    pub location_kind: CapabilityErrorLocationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl CapabilityManifestError {
    pub fn file(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: None,
            message: message.into(),
            location_kind: CapabilityErrorLocationKind::File,
            path: None,
        }
    }

    pub fn field(
        code: impl Into<String>,
        message: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: None,
            message: message.into(),
            location_kind: CapabilityErrorLocationKind::Field,
            path: Some(path.into()),
        }
    }

    pub fn with_severity(mut self, severity: impl Into<String>) -> Self {
        self.severity = Some(severity.into());
        self
    }
}

/// Aggregate errors (multiple issues in one validation pass).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilityManifestErrors {
    pub errors: Vec<CapabilityManifestError>,
}

impl CapabilityManifestErrors {
    pub fn new(errors: Vec<CapabilityManifestError>) -> Self {
        Self { errors }
    }

    pub fn push(&mut self, e: CapabilityManifestError) {
        self.errors.push(e);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl std::fmt::Display for CapabilityManifestErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, e) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "[{}] {}", e.code, e.message)?;
            if let Some(p) = &e.path {
                write!(f, " ({p})")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for CapabilityManifestErrors {}
