//! File search tool with ripgrep priority and built-in fallback.

use agent_diva_core::security::{SecurityPolicy, SharedSecurityPolicy};
use agent_diva_tooling::{Result, Tool};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

/// Search files tool with ripgrep priority and built-in fallback.
pub struct SearchFilesTool {
    security: SharedSecurityPolicy,
}

impl SearchFilesTool {
    /// Create a new search files tool with a security policy.
    pub fn new(security: SharedSecurityPolicy) -> Self {
        Self { security }
    }

    /// Create a new search files tool with default policy for a workspace.
    pub fn for_workspace(workspace: PathBuf) -> Self {
        let policy = Arc::new(SecurityPolicy::new(workspace));
        Self::new(policy)
    }
}

impl Default for SearchFilesTool {
    fn default() -> Self {
        Self::for_workspace(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[derive(Debug, Deserialize)]
struct SearchRequest {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default = "default_max_results")]
    max_results: usize,
    #[serde(default)]
    context_lines: usize,
}

fn default_max_results() -> usize {
    100
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResult {
    matches: Vec<SearchMatch>,
    total_matches: usize,
    truncated: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchMatch {
    file: String,
    line_number: usize,
    line_content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    context_before: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    context_after: Vec<String>,
}

#[async_trait]
impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }

    fn description(&self) -> &str {
        "Search files for a regex pattern using ripgrep with built-in fallback."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regex pattern to search for."
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in. Defaults to workspace root."
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. '*.rs')."
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum results. Default 100, max 500."
                },
                "context_lines": {
                    "type": "integer",
                    "description": "Number of context lines before/after each match. Default 0, max 10."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let request: SearchRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => return Ok(format!("Error: Invalid search arguments: {}", e)),
        };

        if request.pattern.is_empty() {
            return Ok("Error: pattern must not be empty".to_string());
        }

        let max_results = request.max_results.min(500).max(1);
        let context_lines = request.context_lines.min(10);

        if let Err(error) = self.security.can_act() {
            return Ok(format!("Error: {}", error.user_message()));
        }

        // Resolve search path
        let search_path = if let Some(ref p) = request.path {
            if p.is_empty() {
                self.security.workspace_dir().to_path_buf()
            } else {
                match self.security.validate_path(p).await {
                    Ok(path) => path,
                    Err(error) => return Ok(format!("Error: {}", error.user_message())),
                }
            }
        } else {
            self.security.workspace_dir().to_path_buf()
        };

        // Try ripgrep first (only if context_lines is 0, since rg JSON parsing
        // doesn't extract context lines from separate type:"context" entries)
        if context_lines == 0 {
            if let Some(result) = try_ripgrep(
                &request.pattern,
                &search_path,
                request.glob.as_deref(),
                max_results,
            ) {
                return Ok(result);
            }
        }

        // Fall back to built-in search
        Ok(builtin_search(
            &request.pattern,
            &search_path,
            request.glob.as_deref(),
            max_results,
            context_lines,
        ))
    }
}

fn try_ripgrep(
    pattern: &str,
    path: &Path,
    glob: Option<&str>,
    max_results: usize,
) -> Option<String> {
    let rg_path = which::which("rg").ok()?;

    let mut cmd = Command::new(rg_path);
    cmd.arg("--json")
        .arg("--no-heading")
        .arg("--line-number")
        .arg("--max-count")
        .arg(max_results.to_string());

    if let Some(g) = glob {
        cmd.arg("--glob").arg(g);
    }

    cmd.arg("--").arg(pattern).arg(path);

    let output = cmd.output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    // rg may exit non-zero when no matches found, which is fine
    if stdout.trim().is_empty() {
        let result = SearchResult {
            matches: Vec::new(),
            total_matches: 0,
            truncated: false,
        };
        return Some(serde_json::to_string_pretty(&result).unwrap_or_default());
    }

    let mut matches: Vec<SearchMatch> = Vec::new();
    let mut seen: HashSet<(String, usize)> = HashSet::new();

    for line_str in stdout.lines() {
        let line_str = line_str.trim();
        if line_str.is_empty() {
            continue;
        }

        let parsed: Value = match serde_json::from_str(line_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let obj = match parsed.as_object() {
            Some(o) => o,
            None => continue,
        };

        let msg_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if msg_type != "match" {
            continue;
        }

        let data = match obj.get("data") {
            Some(d) => d,
            None => continue,
        };

        let file = data
            .get("path")
            .and_then(|p| p.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let line_num = data
            .get("line_number")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        if line_num == 0 {
            continue;
        }

        let line_text = data
            .get("lines")
            .and_then(|l| l.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_end()
            .to_string();

        let key = (file.clone(), line_num);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);

        if matches.len() >= max_results {
            let result = SearchResult {
                matches,
                total_matches: 0,
                truncated: true,
            };
            return Some(serde_json::to_string_pretty(&result).unwrap_or_default());
        }

        matches.push(SearchMatch {
            file,
            line_number: line_num,
            line_content: line_text,
            context_before: Vec::new(),
            context_after: Vec::new(),
        });
    }

    let total = matches.len();
    let result = SearchResult {
        matches,
        total_matches: total,
        truncated: total >= max_results,
    };
    Some(serde_json::to_string_pretty(&result).unwrap_or_default())
}

fn builtin_search(
    pattern: &str,
    path: &Path,
    glob: Option<&str>,
    max_results: usize,
    context_lines: usize,
) -> String {
    let regex = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => return format!("Error: Invalid regex pattern: {}", e),
    };

    let glob_matcher = match glob {
        Some(g) => match glob::Pattern::new(g) {
            Ok(p) => Some(p),
            Err(e) => return format!("Error: Invalid glob pattern: {}", e),
        },
        None => None,
    };

    let mut matches: Vec<SearchMatch> = Vec::new();

    if let Err(e) = walk_and_search(
        path,
        path,
        &regex,
        glob_matcher.as_ref(),
        max_results,
        context_lines,
        &mut matches,
    ) {
        return format!("Error searching files: {}", e);
    }

    let total = matches.len();
    let truncated = total >= max_results;

    let result = SearchResult {
        matches,
        total_matches: total,
        truncated,
    };
    serde_json::to_string_pretty(&result).unwrap_or_default()
}

fn walk_and_search(
    search_root: &Path,
    dir: &Path,
    regex: &regex::Regex,
    glob_matcher: Option<&glob::Pattern>,
    max_results: usize,
    context_lines: usize,
    matches: &mut Vec<SearchMatch>,
) -> std::io::Result<()> {
    if matches.len() >= max_results {
        return Ok(());
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()), // Skip unreadable directories
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();

        if path.is_dir() {
            // Skip hidden directories and common build artifacts
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.')
                    || name == "target"
                    || name == "node_modules"
                    || name == "__pycache__"
                {
                    continue;
                }
            }
            walk_and_search(
                search_root,
                &path,
                regex,
                glob_matcher,
                max_results,
                context_lines,
                matches,
            )?;
        } else if path.is_file() {
            // Apply glob filter using path relative to search root
            if let Some(matcher) = glob_matcher {
                if let Ok(rel) = path.strip_prefix(search_root) {
                    if !matcher.matches_path(rel) {
                        continue;
                    }
                }
            }

            // Skip likely binary files by extension
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                let binary_exts = [
                    "exe", "dll", "so", "dylib", "bin", "obj", "o", "a", "lib",
                    "zip", "tar", "gz", "bz2", "xz", "7z", "rar",
                    "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg",
                    "mp3", "mp4", "avi", "mov", "wav", "flac",
                    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
                    "ttf", "otf", "woff", "woff2",
                    "wasm",
                ];
                if binary_exts.contains(&ext_lower.as_str()) {
                    continue;
                }
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue, // Skip binary/unreadable files
            };

            let rel_path = match path.strip_prefix(search_root) {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => path.to_string_lossy().to_string(),
            };

            let lines: Vec<&str> = content.lines().collect();

            for (i, line) in lines.iter().enumerate() {
                if regex.is_match(line) {
                    if matches.len() >= max_results {
                        return Ok(());
                    }

                    let line_number = i + 1;
                    let mut context_before = Vec::new();
                    let mut context_after = Vec::new();

                    if context_lines > 0 {
                        let start = i.saturating_sub(context_lines);
                        for j in start..i {
                            context_before.push(lines[j].to_string());
                        }
                        let end = (i + 1 + context_lines).min(lines.len());
                        for j in (i + 1)..end {
                            context_after.push(lines[j].to_string());
                        }
                    }

                    matches.push(SearchMatch {
                        file: rel_path.clone(),
                        line_number,
                        line_content: line.to_string(),
                        context_before,
                        context_after,
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_search_files_basic() {
        let dir = make_temp_dir();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello world\nfoo bar\nhello again\nbaz qux\n").unwrap();

        let tool = SearchFilesTool::for_workspace(dir.path().to_path_buf());
        let result = tokio_test::block_on(tool.execute(json!({
            "pattern": "hello",
            "path": "."
        })))
        .unwrap();

        let search_result: SearchResult = serde_json::from_str(&result).unwrap();
        assert_eq!(search_result.total_matches, 2);
        assert!(!search_result.truncated);
        assert_eq!(search_result.matches.len(), 2);
        assert!(search_result.matches.iter().all(|m| m.line_content.contains("hello")));
    }

    #[test]
    fn test_search_files_regex() {
        let dir = make_temp_dir();
        std::fs::write(
            dir.path().join("data.txt"),
            "line 1\nitem a\nline 2\nitem b\nline 3\n",
        )
        .unwrap();

        let tool = SearchFilesTool::for_workspace(dir.path().to_path_buf());
        let result = tokio_test::block_on(tool.execute(json!({
            "pattern": r"line \d",
            "path": "."
        })))
        .unwrap();

        let search_result: SearchResult = serde_json::from_str(&result).unwrap();
        assert_eq!(search_result.total_matches, 3);
    }

    #[test]
    fn test_search_files_max_results_clamped() {
        let dir = make_temp_dir();
        let mut content = String::new();
        for i in 1..=20 {
            content.push_str(&format!("match line {}\n", i));
        }
        std::fs::write(dir.path().join("many.txt"), content).unwrap();

        let tool = SearchFilesTool::for_workspace(dir.path().to_path_buf());
        let result = tokio_test::block_on(tool.execute(json!({
            "pattern": "match line",
            "path": ".",
            "max_results": 5
        })))
        .unwrap();

        let search_result: SearchResult = serde_json::from_str(&result).unwrap();
        assert!(search_result.total_matches <= 5);
        assert!(search_result.truncated);
    }

    #[test]
    fn test_search_files_no_results() {
        let dir = make_temp_dir();
        std::fs::write(
            dir.path().join("empty.txt"),
            "nothing to see here\n",
        )
        .unwrap();

        let tool = SearchFilesTool::for_workspace(dir.path().to_path_buf());
        let result = tokio_test::block_on(tool.execute(json!({
            "pattern": "nonexistent",
            "path": "."
        })))
        .unwrap();

        let search_result: SearchResult = serde_json::from_str(&result).unwrap();
        assert_eq!(search_result.total_matches, 0);
        assert!(search_result.matches.is_empty());
    }

    #[test]
    fn test_search_files_empty_pattern() {
        let tool = SearchFilesTool::for_workspace(PathBuf::from("."));
        let result = tokio_test::block_on(tool.execute(json!({
            "pattern": ""
        })))
        .unwrap();

        assert!(result.contains("Error"));
        assert!(result.contains("pattern must not be empty"));
    }

    #[test]
    fn test_search_files_with_context() {
        let dir = make_temp_dir();
        std::fs::write(
            dir.path().join("ctx.txt"),
            "line a\nline b\nTARGET\nline d\nline e\n",
        )
        .unwrap();

        let tool = SearchFilesTool::for_workspace(dir.path().to_path_buf());
        let result = tokio_test::block_on(tool.execute(json!({
            "pattern": "TARGET",
            "path": ".",
            "context_lines": 2
        })))
        .unwrap();

        let search_result: SearchResult = serde_json::from_str(&result).unwrap();
        assert_eq!(search_result.total_matches, 1);
        let m = &search_result.matches[0];
        assert_eq!(m.context_before, vec!["line a", "line b"]);
        assert_eq!(m.context_after, vec!["line d", "line e"]);
    }

    #[test]
    fn test_search_files_glob_filter() {
        let dir = make_temp_dir();
        std::fs::write(dir.path().join("alpha.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.path().join("beta.txt"), "fn not_this() {}\n").unwrap();

        let tool = SearchFilesTool::for_workspace(dir.path().to_path_buf());
        let result = tokio_test::block_on(tool.execute(json!({
            "pattern": "fn",
            "path": ".",
            "glob": "*.rs"
        })))
        .unwrap();

        let search_result: SearchResult = serde_json::from_str(&result).unwrap();
        assert_eq!(search_result.total_matches, 1);
        assert!(search_result.matches[0].file.ends_with(".rs"));
    }

    #[test]
    fn test_search_files_default_path() {
        let dir = make_temp_dir();
        std::fs::write(dir.path().join("default.txt"), "unique marker default\n").unwrap();

        let tool = SearchFilesTool::for_workspace(dir.path().to_path_buf());
        let result = tokio_test::block_on(tool.execute(json!({
            "pattern": "unique marker"
        })))
        .unwrap();

        let search_result: SearchResult = serde_json::from_str(&result).unwrap();
        assert_eq!(search_result.total_matches, 1);
    }
}
