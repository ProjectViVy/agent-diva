use agent_diva_core::config::Config;
use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;

#[derive(Debug, Clone, Serialize)]
pub struct DebugBundleReport {
    pub bundle_path: PathBuf,
    pub file_name: String,
    pub run_id: String,
    pub included_files: Vec<String>,
    pub created_at: chrono::DateTime<Utc>,
}

pub fn create_debug_bundle(
    config_dir: &Path,
    config: &Config,
    requested_run_id: Option<&str>,
) -> Result<DebugBundleReport> {
    let run_dir = resolve_run_dir(config_dir, requested_run_id)?;
    let run_id = run_dir
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid debug run directory"))?
        .to_string();
    let bundle_dir = config_dir.join("debug-bundles");
    fs::create_dir_all(&bundle_dir)?;
    let file_name = format!("debug-bundle-{}.zip", run_id);
    let bundle_path = bundle_dir.join(&file_name);
    let file = fs::File::create(&bundle_path)
        .with_context(|| format!("failed to create {}", bundle_path.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut included_files = Vec::new();

    for name in ["manifest.json", "events.jsonl", "raw.jsonl", "gateway.log"] {
        add_file_if_exists(
            &mut zip,
            &run_dir.join(name),
            name,
            options,
            &mut included_files,
        )?;
    }

    let config_summary = redacted_config_value(config)?;
    add_json(
        &mut zip,
        "config-summary.json",
        &config_summary,
        options,
        &mut included_files,
    )?;
    add_json(
        &mut zip,
        "build-info.json",
        &serde_json::json!({
            "package": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
            "created_at": Utc::now(),
            "warning": "This bundle may contain raw secrets and full provider/tool/MCP payloads from the debug run."
        }),
        options,
        &mut included_files,
    )?;

    zip.finish()?;
    Ok(DebugBundleReport {
        bundle_path,
        file_name,
        run_id,
        included_files,
        created_at: Utc::now(),
    })
}

fn resolve_run_dir(config_dir: &Path, requested_run_id: Option<&str>) -> Result<PathBuf> {
    let runs_dir = config_dir.join("debug-runs");
    if let Some(run_id) = requested_run_id {
        let run_dir = runs_dir.join(run_id);
        if run_dir.is_dir() {
            return Ok(run_dir);
        }
        anyhow::bail!("debug run not found: {}", run_id);
    }

    let mut candidates = Vec::new();
    if runs_dir.is_dir() {
        for entry in fs::read_dir(&runs_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let modified = entry.metadata()?.modified()?;
                candidates.push((modified, path));
            }
        }
    }
    candidates.sort_by_key(|(modified, _)| *modified);
    candidates
        .pop()
        .map(|(_, path)| path)
        .ok_or_else(|| anyhow::anyhow!("no debug runs found under {}", runs_dir.display()))
}

fn add_file_if_exists(
    zip: &mut zip::ZipWriter<fs::File>,
    path: &Path,
    zip_name: &str,
    options: FileOptions,
    included_files: &mut Vec<String>,
) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    zip.start_file(zip_name, options)?;
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    zip.write_all(&buffer)?;
    included_files.push(zip_name.to_string());
    Ok(())
}

fn add_json<T: Serialize>(
    zip: &mut zip::ZipWriter<fs::File>,
    zip_name: &str,
    value: &T,
    options: FileOptions,
    included_files: &mut Vec<String>,
) -> Result<()> {
    zip.start_file(zip_name, options)?;
    zip.write_all(&serde_json::to_vec_pretty(value)?)?;
    included_files.push(zip_name.to_string());
    Ok(())
}

fn redacted_config_value(config: &Config) -> Result<serde_json::Value> {
    let mut value = serde_json::to_value(config)?;
    redact_sensitive_value("root", &mut value);
    Ok(value)
}

fn redact_sensitive_value(key: &str, value: &mut serde_json::Value) {
    let lowered = key.to_ascii_lowercase();
    let looks_sensitive = ["api_key", "token", "secret", "password", "authorization"]
        .iter()
        .any(|segment| lowered.contains(segment));

    match value {
        serde_json::Value::Object(map) => {
            for (nested_key, nested_value) in map.iter_mut() {
                redact_sensitive_value(nested_key, nested_value);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_sensitive_value(key, item);
            }
        }
        serde_json::Value::String(text) if looks_sensitive && !text.is_empty() => {
            *text = "***REDACTED***".to_string();
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::config::Config;

    #[test]
    fn bundle_uses_latest_run_and_expected_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let run_dir = temp_dir.path().join("debug-runs").join("debug-run-test");
        fs::create_dir_all(&run_dir).unwrap();
        fs::write(run_dir.join("manifest.json"), "{}").unwrap();
        fs::write(run_dir.join("raw.jsonl"), "{\"api_key\":\"sk-secret\"}\n").unwrap();

        let mut config = Config::default();
        config.providers.openai.api_key = "sk-config-secret".to_string();
        let report = create_debug_bundle(temp_dir.path(), &config, None).unwrap();

        assert!(report.bundle_path.exists());
        assert_eq!(report.run_id, "debug-run-test");
        assert!(report.included_files.contains(&"manifest.json".to_string()));
        assert!(report.included_files.contains(&"raw.jsonl".to_string()));
        assert!(report
            .included_files
            .contains(&"config-summary.json".to_string()));
    }
}
