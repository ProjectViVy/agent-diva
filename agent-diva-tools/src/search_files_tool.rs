//! Search files tool with glob pattern matching and optional content search.
//!
//! This module provides a tool for searching files using glob patterns,
//! with optional literal content search within matched files. Uses
//! the `ignore` crate for gitignore-aware directory traversal (ripgrep-style).

use agent_diva_tooling::base::ToolCapabilities;
use agent_diva_tooling::{Result, Tool};
use async_trait::async_trait;
use globset::Glob;
use ignore::WalkBuilder;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Search files tool supporting glob pattern matching and literal content search.
///
/// Uses `ignore::WalkBuilder` for gitignore-aware traversal and glob patterns
/// for file name matching. Content search is literal (not regex).
pub struct SearchFilesTool {
    workspace_path: PathBuf,
}

impl SearchFilesTool {
    /// Create a new search files tool rooted at the given workspace path.
    pub fn new(workspace_path: PathBuf) -> Self {
        Self { workspace_path }
    }
}

impl Default for SearchFilesTool {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[async_trait]
impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }

    fn description(&self) -> &str {
        "Search for files matching a glob pattern, optionally searching literal content within matched files. Uses gitignore-aware traversal."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match file names (e.g., '*.rs', 'src/**/*.rs')"
                },
                "path": {
                    "type": "string",
                    "description": "Subdirectory to search within (relative to workspace root). Defaults to workspace root."
                },
                "content": {
                    "type": "string",
                    "description": "Optional literal text to search for within matched files"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return Ok("Error: Missing 'pattern' parameter".to_string()),
        };

        let sub_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let content_query = args.get("content").and_then(|v| v.as_str());

        // Resolve the search root
        let search_root = if sub_path.is_empty() || sub_path == "." {
            self.workspace_path.clone()
        } else {
            self.workspace_path.join(sub_path)
        };

        // Check if search root exists
        if !search_root.exists() {
            return Ok(format!(
                "Error: Directory not found: {}",
                sub_path
            ));
        }

        if !search_root.is_dir() {
            return Ok(format!(
                "Error: Not a directory: {}",
                sub_path
            ));
        }

        // Parse glob pattern
        let glob = match Glob::new(pattern) {
            Ok(g) => g.compile_matcher(),
            Err(e) => return Ok(format!("Error: Invalid glob pattern '{}': {}", pattern, e)),
        };

        // Build the gitignore-aware walker
        let mut walker = WalkBuilder::new(&search_root);
        walker.hidden(false); // show hidden files (gitignore still applies)
        walker.git_ignore(true);
        walker.git_global(true);
        walker.git_exclude(true);
        walker.ignore(true);
        walker.require_git(false); // don't require a git repo
        walker.max_depth(None); // no depth limit
        walker.follow_links(false);

        let mut results: Vec<String> = Vec::new();
        let mut total_files_matched = 0usize;
        let mut files_searched = 0usize;

        for entry in walker.build() {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Skip directories
            if entry.file_type().map_or(true, |ft| ft.is_dir()) {
                continue;
            }

            let entry_path = entry.path();

            // Get the relative path for display and glob matching
            let relative = match entry_path.strip_prefix(&search_root) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let relative_str = relative.to_string_lossy();
            // Normalize path separators for glob matching (glob uses / as separator)
            let relative_str = relative_str.replace('\\', "/");

            // Check glob match against the relative path
            if !glob.is_match(&relative_str) {
                continue;
            }

            total_files_matched += 1;

            // If content search is requested, read and search the file
            if let Some(query) = content_query {
                // Skip likely binary files by checking extension hints first
                if is_likely_binary_by_extension(entry_path) {
                    continue;
                }

                match std::fs::read_to_string(entry_path) {
                    Ok(content) => {
                        // Check for null bytes (binary content heuristic)
                        if content.as_bytes().contains(&0u8) {
                            continue;
                        }

                        if content.contains(query) {
                            files_searched += 1;
                            results.push(format!(
                                "{} (matches content)",
                                relative_str
                            ));
                        }
                    }
                    Err(_) => {
                        // Skip unreadable files
                        continue;
                    }
                }
            } else {
                files_searched += 1;
                results.push(relative_str.to_string());
            }
        }

        // Build output
        let mut output = String::new();

        if let Some(query) = content_query {
            output.push_str(&format!(
                "Found {} files matching glob '{}' with content containing \"{}\":\n",
                results.len(),
                pattern,
                query
            ));
        } else {
            output.push_str(&format!(
                "Found {} files matching glob '{}':\n",
                results.len(),
                pattern
            ));
        }

        if results.is_empty() {
            output.push_str("(no matches)");
        } else {
            for r in &results {
                output.push_str(&format!("  {}\n", r));
            }
            output.push_str(&format!(
                "\nTotal: {} file(s) matched, {} file(s) searched (out of {} glob matches)",
                results.len(),
                files_searched,
                total_files_matched
            ));
        }

        Ok(output)
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            requires_filesystem: true,
            is_read_only: true,
            ..ToolCapabilities::default()
        }
    }
}

/// Check if a file is likely binary based on its extension.
///
/// This is a fast pre-filter to avoid reading binary files into a string
/// before the null-byte check.
fn is_likely_binary_by_extension(path: &std::path::Path) -> bool {
    const BINARY_EXTENSIONS: &[&str] = &[
        "exe", "dll", "so", "dylib", "bin", "obj", "o", "a", "lib",
        "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "svgz",
        "mp3", "mp4", "avi", "mov", "mkv", "wav", "flac", "ogg",
        "zip", "tar", "gz", "bz2", "xz", "7z", "rar",
        "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
        "ttf", "otf", "woff", "woff2",
        "wasm", "class", "pyc", "pyo",
    ];

    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| BINARY_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_workspace() -> (TempDir, SearchFilesTool) {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create directory structure
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("tests")).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::create_dir_all(root.join("src/sub")).unwrap();

        // Create .rs files
        std::fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(root.join("src").join("lib.rs"), "pub fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();
        std::fs::write(root.join("src").join("sub").join("mod.rs"), "pub mod sub;").unwrap();
        std::fs::write(root.join("tests").join("test_main.rs"), "// test for main").unwrap();

        // Create non-.rs files
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        std::fs::write(root.join("README.md"), "# Test Project").unwrap();
        std::fs::write(root.join("docs").join("guide.md"), "# Guide").unwrap();

        // Create a file with specific content for content search tests
        std::fs::write(
            root.join("src").join("config.rs"),
            "const SECRET_KEY: &str = \"my-secret-value\";\nconst API_URL: &str = \"https://api.example.com\";",
        ).unwrap();

        let tool = SearchFilesTool::new(root.to_path_buf());
        (temp_dir, tool)
    }

    #[tokio::test]
    async fn test_glob_search_rs_files() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "*.rs" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("main.rs"));
        assert!(result.contains("lib.rs"));
        assert!(result.contains("config.rs"));
        assert!(!result.contains("Cargo.toml"));
        assert!(!result.contains("README.md"));
    }

    #[tokio::test]
    async fn test_glob_search_recursive() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "**/*.rs" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("src/main.rs"));
        assert!(result.contains("src/lib.rs"));
        assert!(result.contains("src/config.rs"));
        assert!(result.contains("src/sub/mod.rs"));
        assert!(result.contains("tests/test_main.rs"));
    }

    #[tokio::test]
    async fn test_glob_search_subdirectory() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "*.rs", "path": "src" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("main.rs"));
        assert!(result.contains("lib.rs"));
        assert!(result.contains("config.rs"));
        // sub/mod.rs may or may not appear depending on walk recursion depth
        // when path is "src" — it's a recursive walk, so sub/ content can appear
    }

    #[tokio::test]
    async fn test_glob_search_with_path_subdir() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "**/*.rs", "path": "src" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("main.rs"));
        assert!(result.contains("lib.rs"));
        assert!(result.contains("sub/mod.rs"));
        assert!(!result.contains("test_main.rs")); // tests/ is outside src/
    }

    #[tokio::test]
    async fn test_content_search() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "*.rs", "content": "SECRET_KEY" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("config.rs"));
        assert!(result.contains("matches content"));
        assert!(!result.contains("main.rs"));
    }

    #[tokio::test]
    async fn test_content_search_no_match() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "*.rs", "content": "NONEXISTENT_STRING_XYZ" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("(no matches)"));
    }

    #[tokio::test]
    async fn test_content_search_markdown_files() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "*.md", "content": "Test" });
        let result = tool.execute(params).await.unwrap();

        // README.md contains "# Test Project"
        assert!(result.contains("README.md"));
        assert!(!result.contains("guide.md")); // "# Guide" does not contain "Test"
    }

    #[tokio::test]
    async fn test_nonexistent_directory() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "*.rs", "path": "nonexistent" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("Error: Directory not found"));
    }

    #[tokio::test]
    async fn test_invalid_glob_pattern() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "[unclosed" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("Error: Invalid glob pattern"));
    }

    #[tokio::test]
    async fn test_missing_pattern_parameter() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({});
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("Error: Missing 'pattern'"));
    }

    #[tokio::test]
    async fn test_glob_search_no_matches() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "*.py" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("(no matches)"));
    }

    #[tokio::test]
    async fn test_search_in_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let tool = SearchFilesTool::new(temp_dir.path().to_path_buf());
        let params = json!({ "pattern": "*.rs" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("(no matches)"));
    }

    #[tokio::test]
    async fn test_default_constructor() {
        let tool = SearchFilesTool::default();
        assert_eq!(tool.name(), "search_files");
    }

    #[tokio::test]
    async fn test_capabilities_are_read_only() {
        let tool = SearchFilesTool::new(PathBuf::from("."));
        let caps = tool.capabilities();
        assert!(caps.is_read_only);
        assert!(caps.requires_filesystem);
        assert!(!caps.requires_network);
        assert!(!caps.can_spawn_subagent);
    }

    #[tokio::test]
    async fn test_glob_search_toml_files() {
        let (_temp, tool) = setup_test_workspace();
        let params = json!({ "pattern": "*.toml" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("Cargo.toml"));
        assert!(!result.contains(".rs"));
    }
}
