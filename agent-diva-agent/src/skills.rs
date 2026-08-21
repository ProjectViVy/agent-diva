//! Machine-wide Skill discovery and prompt-density projection.

use std::path::{Path, PathBuf};

use agent_diva_core::evolution::{SkillDocument, SkillHome, SkillHomeError};
use serde_yaml::Value;

pub use agent_diva_core::evolution::SkillSource;

const ALWAYS_FILE_MAX_CHARS: usize = 4_000;
const ALWAYS_TOTAL_MAX_CHARS: usize = 2_000;

#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub source: SkillSource,
    pub enabled: bool,
    pub always: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub enabled: bool,
    pub always: bool,
}

#[derive(Debug, Clone)]
pub struct SkillsLoader {
    home: SkillHome,
}

impl SkillsLoader {
    pub fn default_builtin_skills_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("skills")
    }

    /// Bind discovery to `{config_dir}/skills`; the workspace is never read.
    pub fn new<P: AsRef<Path>>(config_dir: P, builtin_skills_dir: Option<PathBuf>) -> Self {
        Self {
            home: SkillHome::new(
                config_dir,
                builtin_skills_dir.unwrap_or_else(Self::default_builtin_skills_dir),
            ),
        }
    }

    pub fn skill_home(&self) -> &SkillHome {
        &self.home
    }

    pub fn list_skills(&self, filter_unavailable: bool) -> Vec<SkillInfo> {
        self.home
            .list()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|summary| {
                let document = self.home.read(&summary.slug).ok()?;
                let available = check_requirements(&document);
                if filter_unavailable && !available {
                    return None;
                }
                Some(SkillInfo {
                    name: summary.slug,
                    source: summary.source,
                    enabled: summary.enabled,
                    always: summary.always,
                    available,
                })
            })
            .collect()
    }

    pub fn load_skill(&self, slug: &str) -> Option<String> {
        self.home
            .read_enabled(slug)
            .ok()
            .map(|document| document.markdown)
    }

    /// Stable C1 index: enabled slug plus one-line description, without paths.
    pub fn build_skills_summary(&self) -> String {
        let mut skills = self
            .home
            .list()
            .unwrap_or_default()
            .into_iter()
            .filter(|skill| skill.enabled)
            .collect::<Vec<_>>();
        skills.sort_by_key(|skill| skill.slug.clone());
        if skills.is_empty() {
            return String::new();
        }
        let mut output = String::from("<skills>\n");
        for skill in skills {
            output.push_str("  <skill slug=\"");
            output.push_str(&escape_xml(&skill.slug));
            output.push_str("\">");
            output.push_str(&escape_xml(&single_line(&skill.description)));
            output.push_str("</skill>\n");
        }
        output.push_str("</skills>");
        output
    }

    /// Stable, slug-sorted always bodies under the D3 character budgets.
    pub fn build_always_context(&self) -> String {
        let mut documents = self
            .home
            .list()
            .unwrap_or_default()
            .into_iter()
            .filter(|skill| skill.enabled && skill.always)
            .filter_map(|skill| self.home.read_enabled(&skill.slug).ok())
            .collect::<Vec<_>>();
        documents.sort_by_key(|document| document.summary.slug.clone());

        let mut remaining = ALWAYS_TOTAL_MAX_CHARS;
        let mut rendered = Vec::new();
        for document in documents {
            let body = markdown_body(&document.markdown);
            let body_chars = body.chars().count();
            if body_chars > ALWAYS_FILE_MAX_CHARS || remaining == 0 {
                continue;
            }
            let included = body.chars().take(remaining).collect::<String>();
            if included.is_empty() {
                continue;
            }
            remaining = remaining.saturating_sub(included.chars().count());
            rendered.push(format!(
                "### Skill: {}\n\n{}",
                document.summary.slug,
                included.trim()
            ));
        }
        rendered.join("\n\n---\n\n")
    }

    pub fn get_skill_metadata(&self, slug: &str) -> SkillMetadata {
        let Ok(document) = self.home.read(slug) else {
            return SkillMetadata::default();
        };
        metadata_from_document(&document).unwrap_or_default()
    }

    pub fn get_always_skills(&self) -> Vec<String> {
        let mut skills = self
            .list_skills(true)
            .into_iter()
            .filter(|skill| skill.enabled && skill.always)
            .map(|skill| skill.name)
            .collect::<Vec<_>>();
        skills.sort();
        skills
    }

    pub fn load_skills_for_context(&self, _skill_names: &[String]) -> String {
        self.build_always_context()
    }
}

fn metadata_from_document(document: &SkillDocument) -> Result<SkillMetadata, SkillHomeError> {
    let yaml = frontmatter(&document.markdown)?;
    Ok(SkillMetadata {
        name: yaml.get("name").and_then(Value::as_str).map(str::to_string),
        description: Some(document.summary.description.clone()),
        homepage: yaml
            .get("homepage")
            .and_then(Value::as_str)
            .map(str::to_string),
        enabled: document.summary.enabled,
        always: document.summary.always,
    })
}

fn frontmatter(markdown: &str) -> Result<Value, SkillHomeError> {
    let normalized = markdown.replace("\r\n", "\n");
    let rest = normalized.strip_prefix("---\n").ok_or_else(|| {
        SkillHomeError::InvalidMarkdown("YAML frontmatter opening delimiter is required".into())
    })?;
    let (yaml, _) = rest.split_once("\n---\n").ok_or_else(|| {
        SkillHomeError::InvalidMarkdown("YAML frontmatter closing delimiter is required".into())
    })?;
    serde_yaml::from_str(yaml).map_err(|error| SkillHomeError::InvalidMarkdown(error.to_string()))
}

fn markdown_body(markdown: &str) -> &str {
    markdown
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map_or(markdown, |(_, body)| body)
}

fn check_requirements(document: &SkillDocument) -> bool {
    let Ok(yaml) = frontmatter(&document.markdown) else {
        return false;
    };
    let Some(metadata) = yaml.get("metadata") else {
        return true;
    };
    let parsed_json;
    let metadata = if let Some(raw) = metadata.as_str() {
        parsed_json = serde_json::from_str::<serde_json::Value>(raw).unwrap_or_default();
        &parsed_json
    } else {
        return check_yaml_requirements(metadata);
    };
    let runtime = metadata.get("nanobot").or_else(|| metadata.get("openclaw"));
    let Some(requires) = runtime.and_then(|value| value.get("requires")) else {
        return true;
    };
    json_requirements_available(requires)
}

fn check_yaml_requirements(metadata: &Value) -> bool {
    let runtime = metadata.get("nanobot").or_else(|| metadata.get("openclaw"));
    let Some(requires) = runtime.and_then(|value| value.get("requires")) else {
        return true;
    };
    let bins = requires
        .get("bins")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str);
    if bins.into_iter().any(|bin| which::which(bin).is_err()) {
        return false;
    }
    !requires
        .get("env")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|name| std::env::var(name).is_err())
}

fn json_requirements_available(requires: &serde_json::Value) -> bool {
    let bins = requires
        .get("bins")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str);
    if bins.into_iter().any(|bin| which::which(bin).is_err()) {
        return false;
    }
    !requires
        .get("env")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .any(|name| std::env::var(name).is_err())
}

fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn skill(root: &Path, slug: &str, description: &str, enabled: bool, always: bool, body: &str) {
        let directory = root.join("skills").join(slug);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("SKILL.md"),
            format!(
                "---\ndescription: {description}\nenabled: {enabled}\nalways: {always}\n---\n{body}"
            ),
        )
        .unwrap();
    }

    #[test]
    fn summary_is_dense_sorted_and_has_no_disk_paths() {
        let config = TempDir::new().unwrap();
        let builtin = TempDir::new().unwrap();
        skill(config.path(), "zeta", "  Multi\n line ", true, false, "z");
        skill(config.path(), "alpha", "Alpha", true, false, "a");
        skill(config.path(), "off", "Disabled", false, true, "off");
        let loader = SkillsLoader::new(config.path(), Some(builtin.path().to_path_buf()));
        let summary = loader.build_skills_summary();
        assert!(summary.find("alpha").unwrap() < summary.find("zeta").unwrap());
        assert!(summary.contains("Multi line"));
        assert!(!summary.contains("off"));
        assert!(!summary.contains(config.path().to_string_lossy().as_ref()));
        assert!(!summary.contains("<location>"));
    }

    #[test]
    fn always_context_obeys_per_file_and_total_character_budgets() {
        let config = TempDir::new().unwrap();
        let builtin = TempDir::new().unwrap();
        skill(
            config.path(),
            "alpha",
            "Alpha",
            true,
            true,
            &"甲".repeat(1_500),
        );
        skill(
            config.path(),
            "beta",
            "Beta",
            true,
            true,
            &"乙".repeat(1_500),
        );
        skill(
            config.path(),
            "huge",
            "Huge",
            true,
            true,
            &"丙".repeat(4_001),
        );
        let loader = SkillsLoader::new(config.path(), Some(builtin.path().to_path_buf()));
        let context = loader.build_always_context();
        assert!(context.contains("Skill: alpha"));
        assert!(context.contains("Skill: beta"));
        assert!(!context.contains("Skill: huge"));
        let body_chars = context
            .chars()
            .filter(|character| matches!(character, '甲' | '乙'))
            .count();
        assert_eq!(body_chars, 2_000);
    }

    #[test]
    fn disabled_home_does_not_fall_back_to_builtin() {
        let config = TempDir::new().unwrap();
        let builtin = TempDir::new().unwrap();
        fs::create_dir_all(builtin.path().join("demo")).unwrap();
        fs::write(
            builtin.path().join("demo/SKILL.md"),
            "---\ndescription: Builtin\n---\nbuiltin",
        )
        .unwrap();
        skill(config.path(), "demo", "Home", false, false, "home");
        let loader = SkillsLoader::new(config.path(), Some(builtin.path().to_path_buf()));
        assert!(loader.load_skill("demo").is_none());
        assert!(loader.build_skills_summary().is_empty());
    }
}
