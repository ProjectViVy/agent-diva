//! Skill loading and management
//!
//! Skills are markdown files (SKILL.md) that teach the agent how to use
//! specific tools or perform certain tasks. They contain YAML frontmatter
//! with metadata and markdown content with instructions.
//!
//! **Capability manifest v0 (FR11)** is separate JSON validated in [`crate::capability`]:
//! it declares package-level capability **entries** (`id`, optional `priority`, etc.), while this
//! loader scans **per-skill** `SKILL.md` directories. The two can be composed by upper layers;
//! do not conflate manifest paths with `skills/` directory layout.

use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Skill information
#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub path: PathBuf,
    pub source: SkillSource,
}

/// Skill source location
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Workspace,
    Builtin,
}

impl std::fmt::Display for SkillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillSource::Workspace => write!(f, "workspace"),
            SkillSource::Builtin => write!(f, "builtin"),
        }
    }
}

/// Skill metadata from frontmatter
#[derive(Debug, Clone, Default)]
pub struct SkillMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub always: bool,
    pub metadata: Option<String>,
}

/// Parsed agent-diva metadata from JSON in frontmatter
#[derive(Debug, Clone, Default)]
pub struct SkillRuntimeMetadata {
    pub emoji: Option<String>,
    pub always: bool,
    pub requires_bins: Vec<String>,
    pub requires_env: Vec<String>,
}

/// Skills loader for agent capabilities
pub struct SkillsLoader {
    workspace_skills: PathBuf,
    builtin_skills: PathBuf,
}

impl SkillsLoader {
    fn default_builtin_skills_dir() -> PathBuf {
        // `agent-diva-agent` sits next to `skills/` in the workspace tree.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("skills")
    }

    /// Create a new skills loader
    ///
    /// # Arguments
    ///
    /// * `workspace` - Path to the workspace directory
    /// * `builtin_skills_dir` - Optional path to built-in skills (defaults to bundled skills)
    pub fn new<P: AsRef<Path>>(workspace: P, builtin_skills_dir: Option<PathBuf>) -> Self {
        let workspace = workspace.as_ref();
        let workspace_skills = workspace.join("skills");
        Self {
            workspace_skills,
            builtin_skills: builtin_skills_dir.unwrap_or_else(Self::default_builtin_skills_dir),
        }
    }

    /// List all available skills
    ///
    /// # Arguments
    ///
    /// * `filter_unavailable` - If true, filter out skills with unmet requirements
    ///
    /// # Returns
    ///
    /// List of skill information
    pub fn list_skills(&self, filter_unavailable: bool) -> Vec<SkillInfo> {
        let mut skills = Vec::new();

        // Workspace skills (highest priority)
        if self.workspace_skills.exists() {
            if let Ok(entries) = fs::read_dir(&self.workspace_skills) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let skill_file = entry.path().join("SKILL.md");
                        if skill_file.exists() {
                            if let Some(name) = entry.file_name().to_str() {
                                skills.push(SkillInfo {
                                    name: name.to_string(),
                                    path: skill_file,
                                    source: SkillSource::Workspace,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Built-in skills
        if self.builtin_skills.exists() {
            if let Ok(entries) = fs::read_dir(&self.builtin_skills) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let skill_file = entry.path().join("SKILL.md");
                        if skill_file.exists() {
                            if let Some(name) = entry.file_name().to_str() {
                                // Skip if already in workspace skills
                                if !skills.iter().any(|s| s.name == name) {
                                    skills.push(SkillInfo {
                                        name: name.to_string(),
                                        path: skill_file,
                                        source: SkillSource::Builtin,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Filter by requirements
        if filter_unavailable {
            skills.retain(|s| {
                let meta = self.get_skill_runtime_metadata(&s.name);
                self.check_requirements(&meta)
            });
        }

        skills
    }

    /// Load a skill by name
    ///
    /// # Arguments
    ///
    /// * `name` - Skill name (directory name)
    ///
    /// # Returns
    ///
    /// Skill content or None if not found
    pub fn load_skill(&self, name: &str) -> Option<String> {
        // Check workspace first
        let workspace_skill = self.workspace_skills.join(name).join("SKILL.md");
        if workspace_skill.exists() {
            return fs::read_to_string(workspace_skill).ok();
        }

        // Check built-in
        let builtin_skill = self.builtin_skills.join(name).join("SKILL.md");
        if builtin_skill.exists() {
            return fs::read_to_string(builtin_skill).ok();
        }

        None
    }

    /// Load specific skills for inclusion in agent context
    ///
    /// # Arguments
    ///
    /// * `skill_names` - List of skill names to load
    ///
    /// # Returns
    ///
    /// Formatted skills content
    pub fn load_skills_for_context(&self, skill_names: &[String]) -> String {
        let mut parts = Vec::new();

        for name in skill_names {
            if let Some(content) = self.load_skill(name) {
                let content = Self::strip_frontmatter(&content);
                parts.push(format!("### Skill: {}\n\n{}", name, content));
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join("\n\n---\n\n")
        }
    }

    /// Build a summary of all skills (name, description, path, availability)
    ///
    /// This is used for progressive loading - the agent can read the full
    /// skill content using read_file when needed.
    ///
    /// # Returns
    ///
    /// XML-formatted skills summary
    pub fn build_skills_summary(&self) -> String {
        let all_skills = self.list_skills(false);
        if all_skills.is_empty() {
            return String::new();
        }

        let mut lines = vec!["<skills>".to_string()];

        for skill in all_skills {
            let name = Self::escape_xml(&skill.name);
            let path = skill.path.display().to_string();
            let desc = Self::escape_xml(&self.get_skill_description(&skill.name));
            let meta = self.get_skill_runtime_metadata(&skill.name);
            let available = self.check_requirements(&meta);

            lines.push(format!(
                "  <skill available=\"{}\">",
                if available { "true" } else { "false" }
            ));
            lines.push(format!("    <name>{}</name>", name));
            lines.push(format!("    <description>{}</description>", desc));
            lines.push(format!("    <location>{}</location>", path));

            // Show missing requirements for unavailable skills
            if !available {
                let missing = self.get_missing_requirements(&meta);
                if !missing.is_empty() {
                    lines.push(format!(
                        "    <requires>{}</requires>",
                        Self::escape_xml(&missing)
                    ));
                }
            }

            lines.push("  </skill>".to_string());
        }

        lines.push("</skills>".to_string());
        lines.join("\n")
    }

    /// Get skills marked as always=true that meet requirements
    pub fn get_always_skills(&self) -> Vec<String> {
        let mut result = Vec::new();

        for skill in self.list_skills(true) {
            let metadata = self.get_skill_metadata(&skill.name);
            let runtime_meta = self.get_skill_runtime_metadata(&skill.name);

            if metadata.always || runtime_meta.always {
                result.push(skill.name);
            }
        }

        result
    }

    /// Get metadata from a skill's frontmatter
    ///
    /// # Arguments
    ///
    /// * `name` - Skill name
    ///
    /// # Returns
    ///
    /// Metadata or default if not found
    pub fn get_skill_metadata(&self, name: &str) -> SkillMetadata {
        let content = match self.load_skill(name) {
            Some(c) => c,
            None => return SkillMetadata::default(),
        };

        if !content.starts_with("---") {
            return SkillMetadata::default();
        }

        // Match YAML frontmatter
        let re = Regex::new(r"(?s)^---\n(.*?)\n---").unwrap();
        if let Some(caps) = re.captures(&content) {
            let yaml_content = caps.get(1).unwrap().as_str();
            return Self::parse_yaml_frontmatter(yaml_content);
        }

        SkillMetadata::default()
    }

    /// Get runtime metadata from a skill frontmatter JSON blob.
    fn get_skill_runtime_metadata(&self, name: &str) -> SkillRuntimeMetadata {
        let metadata = self.get_skill_metadata(name);
        if let Some(ref meta_str) = metadata.metadata {
            return Self::parse_runtime_metadata(meta_str);
        }
        SkillRuntimeMetadata::default()
    }

    /// Get the description of a skill
    fn get_skill_description(&self, name: &str) -> String {
        let meta = self.get_skill_metadata(name);
        meta.description.unwrap_or_else(|| name.to_string())
    }

    /// Check if skill requirements are met (bins, env vars)
    fn check_requirements(&self, meta: &SkillRuntimeMetadata) -> bool {
        // Check required binaries
        for bin in &meta.requires_bins {
            if which::which(bin).is_err() {
                return false;
            }
        }

        // Check required environment variables
        for env in &meta.requires_env {
            if std::env::var(env).is_err() {
                return false;
            }
        }

        true
    }

    /// Get a description of missing requirements
    fn get_missing_requirements(&self, meta: &SkillRuntimeMetadata) -> String {
        let mut missing = Vec::new();

        for bin in &meta.requires_bins {
            if which::which(bin).is_err() {
                missing.push(format!("CLI: {}", bin));
            }
        }

        for env in &meta.requires_env {
            if std::env::var(env).is_err() {
                missing.push(format!("ENV: {}", env));
            }
        }

        missing.join(", ")
    }

    /// Remove YAML frontmatter from markdown content
    fn strip_frontmatter(content: &str) -> String {
        if !content.starts_with("---") {
            return content.to_string();
        }

        let re = Regex::new(r"(?s)^---\n.*?\n---\n").unwrap();
        if let Some(m) = re.find(content) {
            return content[m.end()..].trim().to_string();
        }

        content.to_string()
    }

    /// Parse YAML frontmatter (simple key-value parser)
    fn parse_yaml_frontmatter(yaml: &str) -> SkillMetadata {
        let mut metadata = SkillMetadata::default();

        for line in yaml.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');

                match key {
                    "name" => metadata.name = Some(value.to_string()),
                    "description" => metadata.description = Some(value.to_string()),
                    "homepage" => metadata.homepage = Some(value.to_string()),
                    "always" => metadata.always = value == "true",
                    "metadata" => metadata.metadata = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        metadata
    }

    /// Parse runtime metadata JSON from frontmatter.
    /// Supports `nanobot` and `openclaw` keys for compatibility.
    fn parse_runtime_metadata(raw: &str) -> SkillRuntimeMetadata {
        let value: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return SkillRuntimeMetadata::default(),
        };

        let runtime = match value.get("nanobot").or_else(|| value.get("openclaw")) {
            Some(n) => n,
            None => return SkillRuntimeMetadata::default(),
        };

        let mut meta = SkillRuntimeMetadata::default();

        if let Some(emoji) = runtime.get("emoji").and_then(|v| v.as_str()) {
            meta.emoji = Some(emoji.to_string());
        }

        if let Some(always) = runtime.get("always").and_then(|v| v.as_bool()) {
            meta.always = always;
        }

        if let Some(requires) = runtime.get("requires").and_then(|v| v.as_object()) {
            if let Some(bins) = requires.get("bins").and_then(|v| v.as_array()) {
                meta.requires_bins = bins
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }

            if let Some(env) = requires.get("env").and_then(|v| v.as_array()) {
                meta.requires_env = env
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }

        meta
    }

    /// Escape XML special characters
    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skill(dir: &Path, name: &str, content: &str) {
        let skill_dir = dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn test_list_skills() {
        let workspace = TempDir::new().unwrap();
        let builtin = TempDir::new().unwrap();
        let skills_dir = workspace.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        create_test_skill(
            &skills_dir,
            "test-skill",
            "---\nname: test-skill\ndescription: A test skill\n---\n\n# Test\n",
        );

        let loader = SkillsLoader::new(workspace.path(), Some(builtin.path().to_path_buf()));
        let skills = loader.list_skills(false);

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");
        assert_eq!(skills[0].source, SkillSource::Workspace);
    }

    #[test]
    fn test_load_skill() {
        let workspace = TempDir::new().unwrap();
        let skills_dir = workspace.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        let content = "---\nname: test\n---\n\n# Test Content\n";
        create_test_skill(&skills_dir, "test", content);

        let loader = SkillsLoader::new(workspace.path(), None);
        let loaded = loader.load_skill("test");

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), content);
    }

    #[test]
    fn test_strip_frontmatter() {
        let content = "---\nname: test\n---\n\n# Content";
        let stripped = SkillsLoader::strip_frontmatter(content);
        assert_eq!(stripped, "# Content");
    }

    #[test]
    fn test_parse_metadata() {
        let yaml = "name: test\ndescription: A test\nalways: true";
        let meta = SkillsLoader::parse_yaml_frontmatter(yaml);

        assert_eq!(meta.name.unwrap(), "test");
        assert_eq!(meta.description.unwrap(), "A test");
        assert!(meta.always);
    }

    #[test]
    fn test_parse_runtime_metadata_nanobot() {
        let json =
            r#"{"nanobot":{"emoji":"cloud","requires":{"bins":["curl"],"env":["API_KEY"]}}}"#;
        let meta = SkillsLoader::parse_runtime_metadata(json);

        assert_eq!(meta.emoji.unwrap(), "cloud");
        assert_eq!(meta.requires_bins, vec!["curl"]);
        assert_eq!(meta.requires_env, vec!["API_KEY"]);
    }

    #[test]
    fn test_parse_runtime_metadata_openclaw() {
        let json = r#"{"openclaw":{"always":true,"requires":{"bins":["git"]}}}"#;
        let meta = SkillsLoader::parse_runtime_metadata(json);

        assert!(meta.always);
        assert_eq!(meta.requires_bins, vec!["git"]);
    }

    #[test]
    fn test_parse_runtime_metadata_ignores_agent_diva_key() {
        let json = r#"{"agent-diva":{"always":true}}"#;
        let meta = SkillsLoader::parse_runtime_metadata(json);

        assert!(!meta.always);
        assert!(meta.requires_bins.is_empty());
        assert!(meta.requires_env.is_empty());
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(SkillsLoader::escape_xml("<test>"), "&lt;test&gt;");
        assert_eq!(SkillsLoader::escape_xml("a & b"), "a &amp; b");
    }

    #[test]
    fn test_build_skills_summary() {
        let workspace = TempDir::new().unwrap();
        let skills_dir = workspace.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        create_test_skill(
            &skills_dir,
            "weather",
            "---\nname: weather\ndescription: Weather info\n---\n\n# Weather\n",
        );

        let loader = SkillsLoader::new(workspace.path(), None);
        let summary = loader.build_skills_summary();

        assert!(summary.contains("<skills>"));
        assert!(summary.contains("<name>weather</name>"));
        assert!(summary.contains("<description>Weather info</description>"));
    }

    #[test]
    fn test_workspace_overrides_builtin() {
        let workspace = TempDir::new().unwrap();
        let builtin = TempDir::new().unwrap();
        let workspace_skills = workspace.path().join("skills");
        fs::create_dir_all(&workspace_skills).unwrap();

        create_test_skill(
            &workspace_skills,
            "weather",
            "---\nname: weather\ndescription: Workspace Weather\n---\n\n# Workspace\n",
        );
        create_test_skill(
            builtin.path(),
            "weather",
            "---\nname: weather\ndescription: Builtin Weather\n---\n\n# Builtin\n",
        );

        let loader = SkillsLoader::new(workspace.path(), Some(builtin.path().to_path_buf()));
        let summary = loader.build_skills_summary();

        assert!(summary.contains("<description>Workspace Weather</description>"));
        assert!(!summary.contains("Builtin Weather"));
    }

    #[test]
    fn test_default_builtin_dir_loads_skills() {
        let workspace = TempDir::new().unwrap();
        let loader = SkillsLoader::new(workspace.path(), None);
        let skills = loader.list_skills(false);

        assert!(skills.iter().any(|s| s.source == SkillSource::Builtin));
    }
}
