//! Mask registry — scans a `masks/` directory and caches parsed mask files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use super::error::MaskError;
use super::mask_file::MaskFile;

/// Static default mask — allocated once.
static DEFAULT_MASK: LazyLock<MaskFile> = LazyLock::new(MaskFile::default_mask);

/// In-memory registry of all masks discovered under a directory.
///
/// The registry scans for `*.md` files (recursively), parses their YAML
/// frontmatter via [`MaskFile::parse_with_path`], and caches the results.
/// Invalid files are skipped with a warning log.
///
/// The default mask ("我就是我") is always available via [`get`](Self::get)
/// and [`list`](Self::list).
///
/// The registry also tracks which mask is currently active via
/// [`switch_to`](Self::switch_to) and [`switch_off`](Self::switch_off).
#[derive(Debug)]
pub struct MaskRegistry {
    /// Root directory to scan for mask files.
    masks_dir: PathBuf,
    /// Cache keyed by relative path without `.md` extension (e.g. `coding/rust-coder`).
    cache: HashMap<String, MaskFile>,
    /// Name of the currently active mask, or `None` for the default mask.
    current_mask_name: Option<String>,
}

impl MaskRegistry {
    /// Create a new registry by scanning `masks_dir` for `.md` mask files.
    ///
    /// If the directory does not exist the registry starts empty (the default
    /// mask is still available).
    pub fn new(masks_dir: impl Into<PathBuf>) -> Self {
        let masks_dir = masks_dir.into();
        let cache = scan_masks(&masks_dir);
        Self {
            masks_dir,
            cache,
            current_mask_name: None,
        }
    }

    /// Return all known masks **including** the default mask.
    pub fn list(&self) -> Vec<&MaskFile> {
        let mut result: Vec<&MaskFile> = self.cache.values().collect();
        result.push(&DEFAULT_MASK);
        result
    }

    /// Look up a mask by its frontmatter `name` field.
    ///
    /// Returns the default mask when `name` matches [`MaskFile::DEFAULT_NAME`].
    pub fn get(&self, name: &str) -> Option<&MaskFile> {
        if name == MaskFile::DEFAULT_NAME {
            return Some(&DEFAULT_MASK);
        }
        self.cache.values().find(|m| m.frontmatter.name == name)
    }

    /// Re-scan the masks directory, replacing the in-memory cache.
    pub fn reload(&mut self) {
        self.cache = scan_masks(&self.masks_dir);
    }

    /// Switch to a mask by its frontmatter `name` field.
    ///
    /// Returns a reference to the activated [`MaskFile`], or
    /// [`MaskError::MaskNotFound`] if no mask with that name exists.
    pub fn switch_to(&mut self, name: &str) -> Result<&MaskFile, MaskError> {
        // Verify the mask exists (immutable borrow — ends after this block).
        if name != MaskFile::DEFAULT_NAME
            && !self.cache.values().any(|m| m.frontmatter.name == name)
        {
            return Err(MaskError::MaskNotFound {
                name: name.to_string(),
            });
        }
        self.current_mask_name = Some(name.to_string());
        // Borrow of current_mask_name is done; get() borrows &self only.
        Ok(self.get(name).unwrap())
    }

    /// Switch off the current mask, returning to the default identity.
    pub fn switch_off(&mut self) {
        self.current_mask_name = None;
    }

    /// Return a reference to the currently active mask.
    ///
    /// Returns `None` when the default mask is active (no custom mask set).
    pub fn current_mask(&self) -> Option<&MaskFile> {
        let name = self.current_mask_name.as_ref()?;
        self.get(name)
    }

    /// Return the name of the currently active mask, or `None` for default.
    pub fn current_mask_name(&self) -> Option<&str> {
        self.current_mask_name.as_deref()
    }
}

// ---------------------------------------------------------------------------
// Directory scanner
// ---------------------------------------------------------------------------

/// Recursively walk `dir`, parse every `.md` file, and return a cache.
fn scan_masks(dir: &Path) -> HashMap<String, MaskFile> {
    let mut cache = HashMap::new();
    scan_dir(dir, dir, &mut cache);
    cache
}

/// Recursively walk `root` from `current`, inserting parsed masks into `cache`.
fn scan_dir(root: &Path, current: &Path, cache: &mut HashMap<String, MaskFile>) {
    if !current.is_dir() {
        // First call may point at a non-existent path — warn once at the root level.
        if current == root {
            tracing::warn!(dir = %current.display(), "masks directory does not exist");
        }
        return;
    }

    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(dir = %current.display(), error = %e, "failed to read masks directory");
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, "failed to read directory entry");
                continue;
            }
        };

        let path = entry.path();

        if path.is_dir() {
            scan_dir(root, &path, cache);
            continue;
        }

        // Only process .md files
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        // Read file contents
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "failed to read mask file");
                continue;
            }
        };

        // Parse mask
        let mask = match MaskFile::parse_with_path(&content, &path.to_string_lossy()) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "skipping invalid mask file");
                continue;
            }
        };

        // Build cache key: relative path without .md extension
        let key = match path.strip_prefix(root) {
            Ok(rel) => {
                let without_ext = rel.with_extension("");
                without_ext.to_string_lossy().replace('\\', "/")
            }
            Err(_) => path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        };

        cache.insert(key, mask);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper: create a temp directory with mask files and return the path.
    fn setup_masks_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create temp dir");

        // Root-level mask
        fs::write(
            dir.path().join("helper.md"),
            "---\nname: \"助手\"\nicon: \"🤖\"\n---\n\n我是一个助手。",
        )
        .unwrap();

        // Nested subdirectory mask
        fs::create_dir_all(dir.path().join("coding")).unwrap();
        fs::write(
            dir.path().join("coding/rust-coder.md"),
            "---\nname: \"Rust Coder\"\nmodel: \"deepseek-chat\"\n---\n\nYou write Rust.",
        )
        .unwrap();

        // Invalid file (should be skipped)
        fs::write(dir.path().join("broken.md"), "no frontmatter here").unwrap();

        // Non-.md file (should be ignored)
        fs::write(dir.path().join("readme.txt"), "not a mask").unwrap();

        dir
    }

    #[test]
    fn load_masks_from_directory() {
        let dir = setup_masks_dir();
        let registry = MaskRegistry::new(dir.path());

        // Should have 2 parsed masks + 1 default
        let masks = registry.list();
        assert_eq!(masks.len(), 3, "expected 2 parsed + 1 default mask");

        // Check specific masks exist
        assert!(registry.get("助手").is_some());
        assert!(registry.get("Rust Coder").is_some());
    }

    #[test]
    fn missing_directory_returns_default_only() {
        let registry = MaskRegistry::new("/nonexistent/path/to/masks");
        let masks = registry.list();
        assert_eq!(masks.len(), 1);
        assert_eq!(masks[0].frontmatter.name, MaskFile::DEFAULT_NAME);
    }

    #[test]
    fn default_mask_always_available() {
        let dir = setup_masks_dir();
        let registry = MaskRegistry::new(dir.path());

        let default = registry.get(MaskFile::DEFAULT_NAME).expect("default mask");
        assert_eq!(default.frontmatter.name, "我就是我");
    }

    #[test]
    fn skip_invalid_files() {
        let dir = setup_masks_dir();
        let registry = MaskRegistry::new(dir.path());

        // "broken.md" was invalid — should not appear
        assert!(registry.get("broken").is_none());

        // Valid masks still loaded
        assert!(registry.get("助手").is_some());
    }

    #[test]
    fn nested_subdirectory_support() {
        let dir = setup_masks_dir();
        let registry = MaskRegistry::new(dir.path());

        // The nested mask should be discoverable
        let rust_coder = registry.get("Rust Coder").expect("nested mask");
        assert_eq!(rust_coder.frontmatter.model.as_deref(), Some("deepseek-chat"));
    }

    #[test]
    fn reload_refreshes_cache() {
        let dir = setup_masks_dir();
        let mut registry = MaskRegistry::new(dir.path());
        assert!(registry.get("新面具").is_none());

        // Add a new mask file
        fs::write(
            dir.path().join("new.md"),
            "---\nname: \"新面具\"\n---\n\n新内容",
        )
        .unwrap();

        // Before reload — not found
        assert!(registry.get("新面具").is_none());

        registry.reload();

        // After reload — found
        assert!(registry.get("新面具").is_some());
    }

    // ── switch_to / switch_off / current_mask tests ────────────────────

    #[test]
    fn switch_to_valid_mask() {
        let dir = setup_masks_dir();
        let mut registry = MaskRegistry::new(dir.path());

        let mask = registry.switch_to("助手").expect("should find mask");
        assert_eq!(mask.frontmatter.name, "助手");
        assert_eq!(registry.current_mask_name(), Some("助手"));
    }

    #[test]
    fn switch_to_invalid_mask_returns_error() {
        let dir = setup_masks_dir();
        let mut registry = MaskRegistry::new(dir.path());

        let result = registry.switch_to("不存在的面具");
        assert!(result.is_err());
        match result.unwrap_err() {
            MaskError::MaskNotFound { name } => assert_eq!(name, "不存在的面具"),
            other => panic!("expected MaskNotFound, got: {other}"),
        }
        // Should remain on default after failed switch.
        assert!(registry.current_mask_name().is_none());
    }

    #[test]
    fn switch_off_resets_to_default() {
        let dir = setup_masks_dir();
        let mut registry = MaskRegistry::new(dir.path());

        registry.switch_to("助手").expect("switch on");
        assert_eq!(registry.current_mask_name(), Some("助手"));

        registry.switch_off();
        assert!(registry.current_mask_name().is_none());
        assert!(registry.current_mask().is_none());
    }

    #[test]
    fn current_mask_returns_active_mask() {
        let dir = setup_masks_dir();
        let mut registry = MaskRegistry::new(dir.path());

        // Initially no active mask (default).
        assert!(registry.current_mask().is_none());

        // Switch to a custom mask.
        registry.switch_to("Rust Coder").expect("switch");
        let current = registry.current_mask().expect("should have current");
        assert_eq!(current.frontmatter.name, "Rust Coder");
        assert_eq!(current.frontmatter.model.as_deref(), Some("deepseek-chat"));
    }

    #[test]
    fn list_shows_all_masks_including_default() {
        let dir = setup_masks_dir();
        let registry = MaskRegistry::new(dir.path());

        let masks = registry.list();
        let names: Vec<&str> = masks.iter().map(|m| m.frontmatter.name.as_str()).collect();
        assert!(names.contains(&"我就是我"), "default mask missing from list");
        assert!(names.contains(&"助手"), "助手 mask missing from list");
        assert!(names.contains(&"Rust Coder"), "Rust Coder mask missing from list");
    }
}
