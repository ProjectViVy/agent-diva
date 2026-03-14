use agent_diva_agent::skills::{SkillSource, SkillsLoader};
use agent_diva_core::config::ConfigLoader;
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillDto {
    pub name: String,
    pub description: String,
    pub source: String,
    pub available: bool,
    pub active: bool,
    pub path: String,
    pub can_delete: bool,
}

#[derive(Clone)]
pub struct SkillService {
    loader: ConfigLoader,
}

impl SkillService {
    pub fn new(loader: ConfigLoader) -> Self {
        Self { loader }
    }

    pub fn list_skills(&self) -> anyhow::Result<Vec<SkillDto>> {
        let workspace = self.workspace_dir()?;
        let loader = SkillsLoader::new(&workspace, None);
        let available_names: HashSet<String> = loader
            .list_skills(true)
            .into_iter()
            .map(|skill| skill.name)
            .collect();
        let active_names: HashSet<String> = loader.get_always_skills().into_iter().collect();

        let mut skills = loader
            .list_skills(false)
            .into_iter()
            .map(|skill| {
                let description = loader
                    .get_skill_metadata(&skill.name)
                    .description
                    .unwrap_or_else(|| skill.name.clone());
                let source = match skill.source {
                    SkillSource::Workspace => "workspace",
                    SkillSource::Builtin => "builtin",
                };
                SkillDto {
                    name: skill.name.clone(),
                    description,
                    source: source.to_string(),
                    available: available_names.contains(&skill.name),
                    active: active_names.contains(&skill.name),
                    path: skill.path.display().to_string(),
                    can_delete: matches!(skill.source, SkillSource::Workspace),
                }
            })
            .collect::<Vec<_>>();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(skills)
    }

    pub fn upload_skill_zip(&self, file_name: &str, bytes: Vec<u8>) -> anyhow::Result<SkillDto> {
        let workspace = self.workspace_dir()?;
        let skills_dir = workspace.join("skills");
        fs::create_dir_all(&skills_dir).with_context(|| {
            format!("failed to create skills directory {}", skills_dir.display())
        })?;

        let archive_paths = list_archive_entries(&bytes)?;
        let single_root = shared_archive_root(&archive_paths);
        let skill_name = derive_skill_name(file_name, &bytes, single_root.as_deref())?;

        let target_dir = skills_dir.join(&skill_name);
        let tmp_dir = skills_dir.join(format!(".upload-{}-{}", skill_name, std::process::id()));
        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir)
                .with_context(|| format!("failed to clean temp directory {}", tmp_dir.display()))?;
        }
        fs::create_dir_all(&tmp_dir)
            .with_context(|| format!("failed to create temp directory {}", tmp_dir.display()))?;

        extract_archive(&bytes, &tmp_dir, single_root.as_deref())?;

        let skill_file = tmp_dir.join("SKILL.md");
        if !skill_file.exists() {
            let _ = fs::remove_dir_all(&tmp_dir);
            return Err(anyhow!("uploaded zip must contain SKILL.md"));
        }

        if target_dir.exists() {
            fs::remove_dir_all(&target_dir).with_context(|| {
                format!(
                    "failed to replace existing skill directory {}",
                    target_dir.display()
                )
            })?;
        }
        fs::rename(&tmp_dir, &target_dir).with_context(|| {
            format!(
                "failed to move uploaded skill into place: {} -> {}",
                tmp_dir.display(),
                target_dir.display()
            )
        })?;

        self.list_skills()?
            .into_iter()
            .find(|skill| skill.name == skill_name)
            .ok_or_else(|| anyhow!("uploaded skill was not visible after install"))
    }

    pub fn delete_skill(&self, name: &str) -> anyhow::Result<()> {
        let workspace = self.workspace_dir()?;
        let workspace_dir = workspace.join("skills").join(name);
        if workspace_dir.exists() {
            fs::remove_dir_all(&workspace_dir).with_context(|| {
                format!(
                    "failed to delete workspace skill directory {}",
                    workspace_dir.display()
                )
            })?;
            return Ok(());
        }

        let builtin_exists = self
            .list_skills()?
            .into_iter()
            .any(|skill| skill.name == name && skill.source == "builtin");
        if builtin_exists {
            return Err(anyhow!("builtin skills cannot be deleted"));
        }

        Err(anyhow!("skill not found"))
    }

    fn workspace_dir(&self) -> anyhow::Result<PathBuf> {
        let config = self.loader.load()?;
        Ok(expand_tilde(&config.agents.defaults.workspace))
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

fn list_archive_entries(bytes: &[u8]) -> anyhow::Result<Vec<PathBuf>> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).context("failed to open uploaded zip archive")?;
    let mut paths = Vec::new();
    for idx in 0..archive.len() {
        let file = archive
            .by_index(idx)
            .context("failed to inspect zip entry")?;
        let path = PathBuf::from(file.name());
        if !path.as_os_str().is_empty() {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn shared_archive_root(paths: &[PathBuf]) -> Option<String> {
    let mut root: Option<String> = None;
    for path in paths {
        let mut components = path.components();
        let Some(Component::Normal(first)) = components.next() else {
            return None;
        };
        components.next()?;
        let first = first.to_string_lossy().to_string();
        if root.as_deref().is_some_and(|existing| existing != first) {
            return None;
        }
        if root.is_none() {
            root = Some(first);
        }
    }
    root
}

fn derive_skill_name(
    file_name: &str,
    bytes: &[u8],
    archive_root: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(root) = archive_root {
        let root = sanitize_skill_name(root);
        if !root.is_empty() {
            return Ok(root);
        }
    }

    if let Some(name) = skill_name_from_archive(bytes)? {
        let name = sanitize_skill_name(&name);
        if !name.is_empty() {
            return Ok(name);
        }
    }

    let fallback = Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(sanitize_skill_name)
        .unwrap_or_default();
    if fallback.is_empty() {
        return Err(anyhow!("failed to derive skill name from uploaded zip"));
    }
    Ok(fallback)
}

fn sanitize_skill_name(input: &str) -> String {
    let mut out = String::new();
    let mut previous_dash = false;
    for ch in input.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            previous_dash = false;
        } else if matches!(ch, '-' | '_' | ' ') && !previous_dash && !out.is_empty() {
            out.push('-');
            previous_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn skill_name_from_archive(bytes: &[u8]) -> anyhow::Result<Option<String>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .context("failed to reopen uploaded zip archive")?;
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx).context("failed to read zip entry")?;
        let entry_path = Path::new(file.name());
        let Some(file_name) = entry_path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if file_name != "SKILL.md" {
            continue;
        }
        let mut content = String::new();
        file.read_to_string(&mut content)
            .context("failed to read SKILL.md from archive")?;
        return Ok(parse_frontmatter_name(&content));
    }
    Ok(None)
}

fn parse_frontmatter_name(content: &str) -> Option<String> {
    if !content.starts_with("---") {
        return None;
    }
    let mut lines = content.lines();
    let _ = lines.next();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim() == "name" {
            let name = value.trim().trim_matches('"').trim_matches('\'');
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn extract_archive(
    bytes: &[u8],
    target_dir: &Path,
    archive_root: Option<&str>,
) -> anyhow::Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .context("failed to extract uploaded zip archive")?;
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx).context("failed to read zip entry")?;
        let entry_path = Path::new(file.name());
        let relative = normalize_archive_path(entry_path, archive_root)
            .ok_or_else(|| anyhow!("zip contains invalid path: {}", entry_path.display()))?;
        if relative.as_os_str().is_empty() {
            continue;
        }

        let output_path = target_dir.join(&relative);
        if file.name().ends_with('/') {
            fs::create_dir_all(&output_path).with_context(|| {
                format!(
                    "failed to create extracted directory {}",
                    output_path.display()
                )
            })?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory {}", parent.display())
            })?;
        }
        let mut output = fs::File::create(&output_path).with_context(|| {
            format!("failed to create extracted file {}", output_path.display())
        })?;
        std::io::copy(&mut file, &mut output)
            .with_context(|| format!("failed to write extracted file {}", output_path.display()))?;
    }
    Ok(())
}

fn normalize_archive_path(path: &Path, archive_root: Option<&str>) -> Option<PathBuf> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_os_string()),
            Component::CurDir => {}
            _ => return None,
        }
    }

    if let Some(root) = archive_root {
        if parts.first().and_then(|value| value.to_str()) == Some(root) {
            parts.remove(0);
        }
    }

    let mut normalized = PathBuf::new();
    for part in parts {
        normalized.push(part);
    }
    Some(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::config::Config;
    use std::io::Write;
    use tempfile::TempDir;
    use zip::write::FileOptions;

    fn write_skill(dir: &Path, name: &str, content: &str) {
        let skill_dir = dir.join("skills").join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    fn write_config(config_dir: &Path, workspace: &Path) {
        let loader = ConfigLoader::with_dir(config_dir);
        let mut config = Config::default();
        config.agents.defaults.workspace = workspace.display().to_string();
        loader.save(&config).unwrap();
    }

    fn make_zip(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            let options = FileOptions::default();
            for (path, body) in entries {
                writer.start_file(*path, options).unwrap();
                writer.write_all(body.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
        }
        cursor.into_inner()
    }

    #[test]
    fn list_skills_marks_active_and_delete_flags() {
        let config_dir = TempDir::new().unwrap();
        let workspace = TempDir::new().unwrap();
        write_config(config_dir.path(), workspace.path());
        write_skill(
            workspace.path(),
            "active-skill",
            "---\nname: active-skill\ndescription: Active\nmetadata: '{\"nanobot\":{\"always\":true}}'\n---\n\n# Active\n",
        );

        let service = SkillService::new(ConfigLoader::with_dir(config_dir.path()));
        let skills = service.list_skills().unwrap();
        let active = skills
            .iter()
            .find(|skill| skill.name == "active-skill")
            .unwrap();
        assert!(active.active);
        assert!(active.can_delete);
        assert_eq!(active.source, "workspace");
    }

    #[test]
    fn upload_skill_zip_supports_single_root_folder() {
        let config_dir = TempDir::new().unwrap();
        let workspace = TempDir::new().unwrap();
        write_config(config_dir.path(), workspace.path());
        let service = SkillService::new(ConfigLoader::with_dir(config_dir.path()));

        let bytes = make_zip(&[(
            "sample-skill/SKILL.md",
            "---\nname: sample-skill\ndescription: Sample\n---\n\n# Skill\n",
        )]);

        let uploaded = service.upload_skill_zip("sample-skill.zip", bytes).unwrap();
        assert_eq!(uploaded.name, "sample-skill");
        assert!(workspace
            .path()
            .join("skills")
            .join("sample-skill")
            .join("SKILL.md")
            .exists());
    }

    #[test]
    fn upload_skill_zip_supports_flat_layout_and_frontmatter_name() {
        let config_dir = TempDir::new().unwrap();
        let workspace = TempDir::new().unwrap();
        write_config(config_dir.path(), workspace.path());
        let service = SkillService::new(ConfigLoader::with_dir(config_dir.path()));

        let bytes = make_zip(&[(
            "SKILL.md",
            "---\nname: flat-skill\ndescription: Flat\n---\n\n# Skill\n",
        )]);

        let uploaded = service.upload_skill_zip("ignored.zip", bytes).unwrap();
        assert_eq!(uploaded.name, "flat-skill");
    }

    #[test]
    fn upload_skill_zip_rejects_missing_skill_file() {
        let config_dir = TempDir::new().unwrap();
        let workspace = TempDir::new().unwrap();
        write_config(config_dir.path(), workspace.path());
        let service = SkillService::new(ConfigLoader::with_dir(config_dir.path()));

        let err = service
            .upload_skill_zip("invalid.zip", make_zip(&[("README.md", "# nope\n")]))
            .unwrap_err();

        assert!(err.to_string().contains("SKILL.md"));
    }

    #[test]
    fn upload_skill_zip_rejects_path_traversal() {
        let config_dir = TempDir::new().unwrap();
        let workspace = TempDir::new().unwrap();
        write_config(config_dir.path(), workspace.path());
        let service = SkillService::new(ConfigLoader::with_dir(config_dir.path()));

        let err = service
            .upload_skill_zip(
                "bad.zip",
                make_zip(&[("../evil/SKILL.md", "---\nname: bad\n---\n\n# Bad\n")]),
            )
            .unwrap_err();

        assert!(err.to_string().contains("invalid path"));
    }

    #[test]
    fn delete_workspace_skill_and_restore_builtin_view() {
        let config_dir = TempDir::new().unwrap();
        let workspace = TempDir::new().unwrap();
        write_config(config_dir.path(), workspace.path());
        write_skill(
            workspace.path(),
            "weather",
            "---\nname: weather\ndescription: Workspace Weather\n---\n\n# Workspace\n",
        );
        let service = SkillService::new(ConfigLoader::with_dir(config_dir.path()));

        let before = service.list_skills().unwrap();
        let weather = before.iter().find(|skill| skill.name == "weather").unwrap();
        assert_eq!(weather.source, "workspace");

        service.delete_skill("weather").unwrap();

        let after = service.list_skills().unwrap();
        let weather = after.iter().find(|skill| skill.name == "weather").unwrap();
        assert_eq!(weather.source, "builtin");
    }

    #[test]
    fn delete_builtin_skill_is_rejected() {
        let config_dir = TempDir::new().unwrap();
        let workspace = TempDir::new().unwrap();
        write_config(config_dir.path(), workspace.path());
        let service = SkillService::new(ConfigLoader::with_dir(config_dir.path()));

        let err = service.delete_skill("weather").unwrap_err();
        assert!(err.to_string().contains("builtin"));
    }
}
