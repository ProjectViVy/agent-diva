use super::PatchTool;
use agent_diva_core::security::{SecurityLevel, SecurityPolicy};
use agent_diva_tooling::Tool;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

fn create_test_tool() -> (PatchTool, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let tool = PatchTool::new(Arc::new(SecurityPolicy::new(temp_dir.path().to_path_buf())));
    (tool, temp_dir)
}

async fn write_fixture(temp_dir: &TempDir, name: &str, content: &str) {
    tokio::fs::write(temp_dir.path().join(name), content)
        .await
        .unwrap();
}

async fn run_patch(
    tool: &PatchTool,
    path: &str,
    old_text: &str,
    new_text: &str,
    strategy: &str,
) -> String {
    tool.execute(json!({
        "path": path,
        "old_text": old_text,
        "new_text": new_text,
        "match_strategy": strategy
    }))
    .await
    .unwrap()
}

#[tokio::test]
async fn test_patch_tool_exact_strategy() {
    let (tool, temp_dir) = create_test_tool();
    write_fixture(&temp_dir, "demo.txt", "alpha\nbeta\ngamma\n").await;

    let result = run_patch(&tool, "demo.txt", "beta", "delta", "Exact").await;
    assert!(result.contains("Successfully patched"));
    assert!(result.contains("Strategy: Exact"));

    let content = tokio::fs::read_to_string(temp_dir.path().join("demo.txt"))
        .await
        .unwrap();
    assert_eq!(content, "alpha\ndelta\ngamma\n");
}

#[tokio::test]
async fn test_patch_tool_trim_whitespace_strategy() {
    let (tool, temp_dir) = create_test_tool();
    write_fixture(&temp_dir, "demo.txt", "start\n    value = 1   \nend\n").await;

    let result = run_patch(
        &tool,
        "demo.txt",
        "value = 1",
        "value = 2",
        "TrimWhitespace",
    )
    .await;
    assert!(result.contains("Strategy: TrimWhitespace"));

    let content = tokio::fs::read_to_string(temp_dir.path().join("demo.txt"))
        .await
        .unwrap();
    assert_eq!(content, "start\nvalue = 2\nend\n");
}

#[tokio::test]
async fn test_patch_tool_normalize_whitespace_strategy() {
    let (tool, temp_dir) = create_test_tool();
    write_fixture(&temp_dir, "demo.txt", "alpha\nvalue    =    1\nomega\n").await;

    let result = run_patch(
        &tool,
        "demo.txt",
        "value = 1",
        "value = 3",
        "NormalizeWhitespace",
    )
    .await;
    assert!(result.contains("Strategy: NormalizeWhitespace"));

    let content = tokio::fs::read_to_string(temp_dir.path().join("demo.txt"))
        .await
        .unwrap();
    assert_eq!(content, "alpha\nvalue = 3\nomega\n");
}

#[tokio::test]
async fn test_patch_tool_case_insensitive_strategy() {
    let (tool, temp_dir) = create_test_tool();
    write_fixture(&temp_dir, "demo.txt", "Hello WORLD\n").await;

    let result = run_patch(
        &tool,
        "demo.txt",
        "hello world",
        "Hello Rust",
        "CaseInsensitive",
    )
    .await;
    assert!(result.contains("Strategy: CaseInsensitive"));

    let content = tokio::fs::read_to_string(temp_dir.path().join("demo.txt"))
        .await
        .unwrap();
    assert_eq!(content, "Hello Rust");
}

#[tokio::test]
async fn test_patch_tool_indent_tolerance_strategy() {
    let (tool, temp_dir) = create_test_tool();
    write_fixture(
        &temp_dir,
        "demo.txt",
        "fn main() {\n        println!(\"hello\");\n}\n",
    )
    .await;

    let result = run_patch(
        &tool,
        "demo.txt",
        "println!(\"hello\");",
        "println!(\"patched\");",
        "IndentTolerance",
    )
    .await;
    assert!(result.contains("Strategy: IndentTolerance"));

    let content = tokio::fs::read_to_string(temp_dir.path().join("demo.txt"))
        .await
        .unwrap();
    assert_eq!(content, "fn main() {\nprintln!(\"patched\");\n}\n");
}

#[tokio::test]
async fn test_patch_tool_fuzzy_strategy() {
    let (tool, temp_dir) = create_test_tool();
    write_fixture(&temp_dir, "demo.txt", "const name = \"patcher\";\n").await;

    let result = run_patch(
        &tool,
        "demo.txt",
        "const name = \"pacher\";",
        "const name = \"patched\";",
        "Fuzzy",
    )
    .await;
    assert!(result.contains("Strategy: Fuzzy"));

    let content = tokio::fs::read_to_string(temp_dir.path().join("demo.txt"))
        .await
        .unwrap();
    assert_eq!(content, "const name = \"patched\";\n");
}

#[tokio::test]
async fn test_patch_tool_line_based_strategy() {
    let (tool, temp_dir) = create_test_tool();
    write_fixture(&temp_dir, "demo.txt", "line-a\nline-b\nline-c\n").await;

    let result = run_patch(
        &tool,
        "demo.txt",
        "line-b\nline-c",
        "line-b\nline-z",
        "LineBased",
    )
    .await;
    assert!(result.contains("Strategy: LineBased"));

    let content = tokio::fs::read_to_string(temp_dir.path().join("demo.txt"))
        .await
        .unwrap();
    assert_eq!(content, "line-a\nline-b\nline-z\n");
}

#[tokio::test]
async fn test_patch_tool_partial_match_strategy() {
    let (tool, temp_dir) = create_test_tool();
    write_fixture(
        &temp_dir,
        "demo.txt",
        "before\nneedle target section\nafter\n",
    )
    .await;

    let result = run_patch(
        &tool,
        "demo.txt",
        "target",
        "replaced section",
        "PartialMatch",
    )
    .await;
    assert!(result.contains("Strategy: PartialMatch"));

    let content = tokio::fs::read_to_string(temp_dir.path().join("demo.txt"))
        .await
        .unwrap();
    assert_eq!(content, "before\nreplaced section\nafter\n");
}

#[tokio::test]
async fn test_patch_tool_regex_strategy() {
    let (tool, temp_dir) = create_test_tool();
    write_fixture(&temp_dir, "demo.txt", "version = 12\n").await;

    let result = run_patch(&tool, "demo.txt", r"version = \d+", "version = 13", "Regex").await;
    assert!(result.contains("Strategy: Regex"));

    let content = tokio::fs::read_to_string(temp_dir.path().join("demo.txt"))
        .await
        .unwrap();
    assert_eq!(content, "version = 13\n");
}

#[tokio::test]
async fn test_patch_tool_rejects_ambiguous_match() {
    let (tool, temp_dir) = create_test_tool();
    write_fixture(&temp_dir, "demo.txt", "repeat\nrepeat\n").await;

    let result = run_patch(&tool, "demo.txt", "repeat", "once", "Exact").await;
    assert!(result.contains("Error:"));
    assert!(result.contains("2 locations"));
}

#[tokio::test]
async fn test_patch_tool_respects_read_only_mode() {
    let temp_dir = TempDir::new().unwrap();
    let tool = PatchTool::new(Arc::new(SecurityPolicy::from_level(
        temp_dir.path().to_path_buf(),
        SecurityLevel::Paranoid,
    )));
    write_fixture(&temp_dir, "demo.txt", "x\n").await;

    let result = run_patch(&tool, "demo.txt", "x", "y", "Exact").await;
    assert!(result.contains("read-only"));
}

#[tokio::test]
async fn test_patch_tool_blocks_path_escape() {
    let (tool, _temp_dir) = create_test_tool();
    let result = run_patch(&tool, "../demo.txt", "x", "y", "Exact").await;
    assert!(result.contains("outside the allowed workspace") || result.contains("forbidden"));
}
