//! Machine-wide Skill Home service and ZIP ingestion.

use std::{
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

use agent_diva_agent::skills::SkillsLoader;
use agent_diva_core::{
    config::ConfigLoader,
    evolution::{validate_skill_slug, SkillDocument, SkillHome, SkillHomeError, SkillSource},
    security::validate_skill_zip_size,
};
use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillDto {
    pub slug: String,
    pub description: String,
    pub source: String,
    pub enabled: bool,
    pub always: bool,
    pub available: bool,
    pub content_hash: String,
    pub updated_at: DateTime<Utc>,
    pub can_hard_delete: bool,
    #[serde(default)]
    pub evolution_managed: bool,
    // Compatibility aliases for the pre-S4 Settings surface. They are not an authority.
    pub name: String,
    pub active: bool,
    pub path: String,
    pub can_delete: bool,
}

impl From<SkillDocument> for SkillDto {
    fn from(document: SkillDocument) -> Self {
        let summary = document.summary;
        let source = match summary.source {
            SkillSource::Home => "home",
            SkillSource::Builtin => "builtin",
        };
        Self {
            slug: summary.slug.clone(),
            description: summary.description,
            source: source.to_string(),
            enabled: summary.enabled,
            always: summary.always,
            available: summary.available,
            content_hash: summary.content_hash,
            updated_at: summary.updated_at,
            can_hard_delete: summary.can_hard_delete,
            evolution_managed: summary.evolution_managed,
            name: summary.slug,
            active: summary.enabled,
            path: String::new(),
            can_delete: summary.can_hard_delete,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillUploadResult {
    pub skill: SkillDto,
    pub changed: bool,
}

#[derive(Clone)]
pub struct SkillService {
    home: SkillHome,
}

impl SkillService {
    pub fn new(loader: ConfigLoader) -> Self {
        Self {
            home: SkillHome::new(
                loader.config_dir(),
                SkillsLoader::default_builtin_skills_dir(),
            ),
        }
    }

    pub fn from_home(home: SkillHome) -> Self {
        Self { home }
    }

    pub fn list_skills(&self) -> anyhow::Result<Vec<SkillDto>> {
        self.home
            .list()?
            .into_iter()
            .map(|summary| self.home.read(&summary.slug).map(SkillDto::from))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn upload_skill_zip(&self, file_name: &str, bytes: Vec<u8>) -> anyhow::Result<SkillDto> {
        self.upload_skill_zip_with_change(file_name, bytes)
            .map(|result| result.skill)
    }

    pub fn upload_skill_zip_with_change(
        &self,
        file_name: &str,
        bytes: Vec<u8>,
    ) -> anyhow::Result<SkillUploadResult> {
        let package = parse_zip_package(file_name, &bytes)?;
        let document = self
            .home
            .install_new(&package.slug, &package.markdown, &package.files)?;
        Ok(SkillUploadResult {
            skill: document.into(),
            changed: true,
        })
    }

    /// Installs a skills.sh marketplace snapshot (file path + UTF-8 contents).
    ///
    /// Mirrors the ZIP upload guards: total size limit, path traversal and
    /// `history/` rejection, and a required root `SKILL.md`.
    pub fn install_marketplace_snapshot(
        &self,
        slug: &str,
        files: &[(String, String)],
    ) -> anyhow::Result<SkillDto> {
        validate_skill_slug(slug)?;
        let total = files
            .iter()
            .map(|(_, contents)| contents.len() as u64)
            .sum();
        validate_skill_zip_size(total).map_err(|error| anyhow!(error))?;

        let mut markdown = None;
        let mut package_files = Vec::new();
        for (path, contents) in files {
            let relative = normalize_snapshot_path(path)
                .ok_or_else(|| anyhow!("marketplace snapshot contains invalid path: {path}"))?;
            if relative
                .components()
                .any(|part| part.as_os_str() == "history")
            {
                return Err(
                    SkillHomeError::InvalidPackagePath(relative.display().to_string()).into(),
                );
            }
            if relative == Path::new("SKILL.md") {
                markdown = Some(contents.clone());
            } else {
                package_files.push((relative, contents.clone().into_bytes()));
            }
        }
        let markdown =
            markdown.ok_or_else(|| anyhow!("marketplace snapshot must contain SKILL.md"))?;
        let document = self.home.install_new(slug, &markdown, &package_files)?;
        Ok(document.into())
    }

    /// Compatibility delete used by the legacy Settings command. New APIs require CAS.
    pub fn delete_skill(&self, name: &str) -> anyhow::Result<()> {
        let document = self.home.read(name)?;
        self.home
            .hard_delete(name, &document.summary.content_hash)
            .map_err(Into::into)
    }

    pub fn home(&self) -> &SkillHome {
        &self.home
    }

    #[cfg(test)]
    fn with_builtin_skills_dir(loader: ConfigLoader, builtin_skills_dir: PathBuf) -> Self {
        Self {
            home: SkillHome::new(loader.config_dir(), builtin_skills_dir),
        }
    }
}

struct ZipPackage {
    slug: String,
    markdown: String,
    files: Vec<(PathBuf, Vec<u8>)>,
}

fn parse_zip_package(file_name: &str, bytes: &[u8]) -> anyhow::Result<ZipPackage> {
    validate_skill_zip_size(bytes.len() as u64).map_err(|error| anyhow!(error))?;
    let paths = list_archive_entries(bytes)?;
    let archive_root = shared_archive_root(&paths);
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).context("failed to open uploaded zip archive")?;
    let mut markdown = None;
    let mut files = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .context("failed to read zip entry")?;
        let entry_path = Path::new(entry.name());
        let relative = normalize_archive_path(entry_path, archive_root.as_deref())
            .ok_or_else(|| anyhow!("zip contains invalid path: {}", entry_path.display()))?;
        if relative.as_os_str().is_empty() || entry.is_dir() {
            continue;
        }
        if relative
            .components()
            .any(|part| part.as_os_str() == "history")
        {
            return Err(SkillHomeError::InvalidPackagePath(relative.display().to_string()).into());
        }
        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .context("failed to read uploaded skill file")?;
        if relative == Path::new("SKILL.md") {
            markdown =
                Some(String::from_utf8(content).context("uploaded SKILL.md must be valid UTF-8")?);
        } else {
            files.push((relative, content));
        }
    }

    let markdown = markdown.ok_or_else(|| anyhow!("uploaded zip must contain SKILL.md"))?;
    let slug = derive_slug(file_name, archive_root.as_deref(), &markdown)?;
    validate_skill_slug(&slug)?;
    Ok(ZipPackage {
        slug,
        markdown,
        files,
    })
}

fn list_archive_entries(bytes: &[u8]) -> anyhow::Result<Vec<PathBuf>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .context("failed to inspect uploaded zip archive")?;
    let mut paths = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .context("failed to inspect zip entry")?;
        let path = PathBuf::from(entry.name());
        if normalize_archive_path(&path, None).is_none() {
            return Err(anyhow!("zip contains invalid path: {}", path.display()));
        }
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
        let Component::Normal(first) = components.next()? else {
            return None;
        };
        components.next()?;
        let first = first.to_string_lossy().to_string();
        if root.as_deref().is_some_and(|existing| existing != first) {
            return None;
        }
        root.get_or_insert(first);
    }
    root
}

fn derive_slug(
    file_name: &str,
    archive_root: Option<&str>,
    markdown: &str,
) -> anyhow::Result<String> {
    if let Some(root) = archive_root {
        validate_skill_slug(root)?;
        return Ok(root.to_string());
    }
    if let Some(name) = frontmatter_name(markdown) {
        validate_skill_slug(&name)?;
        return Ok(name);
    }
    let slug = Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("failed to derive skill slug from uploaded zip"))?;
    validate_skill_slug(slug)?;
    Ok(slug.to_string())
}

fn frontmatter_name(markdown: &str) -> Option<String> {
    let body = markdown.strip_prefix("---\n")?;
    let (yaml, _) = body.split_once("\n---")?;
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).ok()?;
    value
        .as_mapping()?
        .get(serde_yaml::Value::String("name".to_string()))?
        .as_str()
        .map(ToString::to_string)
}

fn normalize_snapshot_path(path: &str) -> Option<PathBuf> {
    normalize_archive_path(Path::new(path), None)
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
    if archive_root.is_some_and(|root| parts.first().and_then(|value| value.to_str()) == Some(root))
    {
        parts.remove(0);
    }
    Some(parts.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use zip::write::FileOptions;

    fn make_zip(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            for (path, body) in entries {
                writer.start_file(*path, FileOptions::default()).unwrap();
                writer.write_all(body.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
        }
        cursor.into_inner()
    }

    fn markdown(name: &str) -> String {
        format!("---\nname: {name}\ndescription: test skill\n---\nbody\n")
    }

    #[test]
    fn installs_only_into_config_skill_home_and_records_history() {
        let config = TempDir::new().unwrap();
        let builtin = TempDir::new().unwrap();
        let service = SkillService::with_builtin_skills_dir(
            ConfigLoader::with_dir(config.path()),
            builtin.path().to_path_buf(),
        );
        let result = service
            .upload_skill_zip(
                "research.zip",
                make_zip(&[("SKILL.md", &markdown("research"))]),
            )
            .unwrap();
        assert_eq!(result.slug, "research");
        assert!(config.path().join("skills/research/SKILL.md").is_file());
        assert!(config.path().join("skills/research/history/1.md").is_file());
    }

    #[test]
    fn zip_cannot_overwrite_or_forge_history() {
        let config = TempDir::new().unwrap();
        let builtin = TempDir::new().unwrap();
        let service = SkillService::with_builtin_skills_dir(
            ConfigLoader::with_dir(config.path()),
            builtin.path().to_path_buf(),
        );
        let bytes = make_zip(&[("research/SKILL.md", &markdown("research"))]);
        service
            .upload_skill_zip("research.zip", bytes.clone())
            .unwrap();
        assert!(service.upload_skill_zip("research.zip", bytes).is_err());
        let forged = make_zip(&[
            ("research/SKILL.md", &markdown("research")),
            ("research/history/9.md", "forged"),
        ]);
        assert!(service.upload_skill_zip("research.zip", forged).is_err());
    }

    #[test]
    fn rejects_traversal_and_invalid_slug() {
        let traversal = make_zip(&[("../SKILL.md", &markdown("research"))]);
        assert!(parse_zip_package("research.zip", &traversal).is_err());
        let invalid = make_zip(&[("SKILL.md", &markdown("Bad Name"))]);
        assert!(parse_zip_package("Bad Name.zip", &invalid).is_err());
    }

    fn snapshot_service() -> (TempDir, SkillService) {
        let config = TempDir::new().unwrap();
        let builtin = TempDir::new().unwrap();
        let service = SkillService::with_builtin_skills_dir(
            ConfigLoader::with_dir(config.path()),
            builtin.path().to_path_buf(),
        );
        (config, service)
    }

    #[test]
    fn installs_marketplace_snapshot_into_skill_home() {
        let (config, service) = snapshot_service();
        let files = vec![
            ("SKILL.md".to_string(), markdown("caveman-commit")),
            ("README.md".to_string(), "readme".to_string()),
        ];
        let skill = service
            .install_marketplace_snapshot("caveman-commit", &files)
            .unwrap();
        assert_eq!(skill.slug, "caveman-commit");
        assert!(config
            .path()
            .join("skills/caveman-commit/SKILL.md")
            .is_file());
        assert!(config
            .path()
            .join("skills/caveman-commit/README.md")
            .is_file());
    }

    #[test]
    fn snapshot_install_rejects_history_traversal_and_missing_skill_md() {
        let (_config, service) = snapshot_service();
        let with_history = vec![
            ("SKILL.md".to_string(), markdown("research")),
            ("history/9.md".to_string(), "forged".to_string()),
        ];
        assert!(service
            .install_marketplace_snapshot("research", &with_history)
            .is_err());

        let traversal = vec![
            ("SKILL.md".to_string(), markdown("research")),
            ("../escape.md".to_string(), "escape".to_string()),
        ];
        assert!(service
            .install_marketplace_snapshot("research", &traversal)
            .is_err());

        let no_skill_md = vec![("README.md".to_string(), "readme".to_string())];
        assert!(service
            .install_marketplace_snapshot("research", &no_skill_md)
            .is_err());

        assert!(service
            .install_marketplace_snapshot(
                "Bad Name",
                &[("SKILL.md".to_string(), "body".to_string())]
            )
            .is_err());
    }
}
