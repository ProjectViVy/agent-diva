use serde_json::Value;

pub const LEGACY_CONFIG_VERSION: u32 = 1;
pub const CURRENT_CONFIG_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationOutcome {
    pub value: Value,
    pub original_version: u32,
    pub final_version: u32,
}

impl MigrationOutcome {
    pub fn changed(&self) -> bool {
        self.original_version != self.final_version
    }
}

pub fn migrate_config_value(mut value: Value) -> crate::Result<MigrationOutcome> {
    let original_version = detect_config_version(&value)?;
    let mut version = original_version;

    while version < CURRENT_CONFIG_VERSION {
        value = match version {
            LEGACY_CONFIG_VERSION => migrate_v1_to_v2(value)?,
            unsupported => {
                return Err(crate::Error::Validation(format!(
                    "unsupported config_version {}",
                    unsupported
                )));
            }
        };
        version = detect_config_version(&value)?;
    }

    if version > CURRENT_CONFIG_VERSION {
        return Err(crate::Error::Validation(format!(
            "config_version {} is newer than supported {}",
            version, CURRENT_CONFIG_VERSION
        )));
    }

    Ok(MigrationOutcome {
        value,
        original_version,
        final_version: version,
    })
}

fn detect_config_version(value: &Value) -> crate::Result<u32> {
    let Some(object) = value.as_object() else {
        return Err(crate::Error::Validation(
            "config root must be a JSON object".to_string(),
        ));
    };

    let Some(version_value) = object.get("config_version") else {
        return Ok(LEGACY_CONFIG_VERSION);
    };

    let Some(version_u64) = version_value.as_u64() else {
        return Err(crate::Error::Validation(
            "config_version must be an unsigned integer".to_string(),
        ));
    };

    u32::try_from(version_u64).map_err(|_| {
        crate::Error::Validation(format!("config_version {} exceeds u32 range", version_u64))
    })
}

fn migrate_v1_to_v2(value: Value) -> crate::Result<Value> {
    let mut object = value
        .as_object()
        .cloned()
        .ok_or_else(|| crate::Error::Validation("config root must be a JSON object".to_string()))?;

    object.insert(
        "config_version".to_string(),
        Value::Number(serde_json::Number::from(CURRENT_CONFIG_VERSION)),
    );

    Ok(Value::Object(object))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::Config;
    use crate::heartbeat::types::{
        DEFAULT_HEARTBEAT_DECIDE_BACKOFF_MS, DEFAULT_HEARTBEAT_DECIDE_MAX_BACKOFF_MS,
        DEFAULT_HEARTBEAT_DECIDE_MAX_RETRIES,
    };

    #[test]
    fn migrate_v1_adds_current_config_version() {
        let value = serde_json::json!({
            "agents": {
                "defaults": {
                    "workspace": "~/workspace",
                    "provider": "deepseek",
                    "model": "deepseek-chat",
                    "max_tokens": 8192,
                    "temperature": 0.7,
                    "max_tool_iterations": 20
                }
            },
            "channels": {},
            "providers": {},
            "gateway": {},
            "tools": {}
        });

        let migrated = migrate_config_value(value).unwrap();
        assert_eq!(migrated.original_version, LEGACY_CONFIG_VERSION);
        assert_eq!(migrated.final_version, CURRENT_CONFIG_VERSION);
        assert_eq!(migrated.value["config_version"], CURRENT_CONFIG_VERSION);

        let config: Config = serde_json::from_value(migrated.value).unwrap();
        assert_eq!(
            config.heartbeat.decide_max_retries,
            DEFAULT_HEARTBEAT_DECIDE_MAX_RETRIES
        );
        assert_eq!(
            config.heartbeat.decide_backoff_ms,
            DEFAULT_HEARTBEAT_DECIDE_BACKOFF_MS
        );
        assert_eq!(
            config.heartbeat.decide_max_backoff_ms,
            DEFAULT_HEARTBEAT_DECIDE_MAX_BACKOFF_MS
        );
    }

    #[test]
    fn migrate_rejects_newer_unsupported_version() {
        let err = migrate_config_value(serde_json::json!({"config_version": 99})).unwrap_err();
        assert!(err.to_string().contains("newer than supported"));
    }
}
