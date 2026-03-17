//! Output sanitization utilities for tool results.
//!
//! Tool outputs may contain control characters, ANSI escape sequences, or other
//! characters that could cause JSON parsing errors when sent to LLM providers.
//! This module provides utilities to clean such outputs.

use regex::Regex;
use std::sync::OnceLock;

/// Maximum length for tool results (in characters) to prevent oversized API requests.
/// This helps avoid 400 errors from LLM providers when the request body is too large.
pub const MAX_TOOL_RESULT_CHARS: usize = 80_000;

/// Maximum length for file content returned by read_file tool.
/// Files larger than this will be truncated with a preview.
pub const MAX_FILE_CONTENT_CHARS: usize = 60_000;

/// Preview length to show when truncating file content.
pub const FILE_PREVIEW_CHARS: usize = 10_000;

/// Remove ANSI escape sequences from text.
/// Handles common sequences like colors, cursor movement, screen clearing, etc.
fn remove_ansi_sequences(s: &str) -> String {
    // Standard CSI sequences: ESC [ followed by parameters and a letter
    static CSI_RE: OnceLock<Regex> = OnceLock::new();
    let csi_re =
        CSI_RE.get_or_init(|| Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").expect("invalid CSI regex"));
    let result = csi_re.replace_all(s, "");

    // OSC sequences: ESC ] ... BEL or ESC ] ... ST
    static OSC_RE: OnceLock<Regex> = OnceLock::new();
    let osc_re = OSC_RE.get_or_init(|| {
        Regex::new(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)").expect("invalid OSC regex")
    });
    osc_re.replace_all(&result, "").to_string()
}

/// Remove control characters from text, preserving common whitespace.
/// Control characters are: 0x00-0x1F (except tab, newline, CR) and 0x7F (DEL).
fn remove_control_chars(s: &str) -> String {
    s.chars()
        .filter(|&c| {
            let cp = c as u32;
            // Keep tab (0x09), newline (0x0A), carriage return (0x0D)
            // Keep normal printable chars and extended unicode
            !(cp < 0x20 && cp != 0x09 && cp != 0x0A && cp != 0x0D) && cp != 0x7F
        })
        .collect()
}

/// Sanitize output for safe JSON serialization.
///
/// This function removes:
/// - ANSI escape sequences (colors, cursor control, etc.)
/// - Control characters (NULL, BEL, etc.)
///
/// It preserves:
/// - Normal whitespace (tab, newline, carriage return)
/// - Unicode characters (Chinese, emoji, etc.)
/// - All other printable text
pub fn sanitize_for_json(s: &str) -> String {
    let result = remove_ansi_sequences(s);
    remove_control_chars(&result)
}

/// Truncate a string to a maximum character length with a truncation notice.
/// Uses character count (not bytes) to handle multi-byte UTF-8 characters correctly.
pub fn truncate_string(s: &str, max_chars: usize, notice: &str) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}{}", truncated, notice)
    }
}

/// Truncate file content for safe return from read_file tool.
/// Shows a preview followed by truncation notice.
pub fn truncate_file_content(content: &str) -> String {
    let char_count = content.chars().count();
    if char_count <= MAX_FILE_CONTENT_CHARS {
        content.to_string()
    } else {
        let preview: String = content.chars().take(FILE_PREVIEW_CHARS).collect();
        format!(
            "{}\n\n... [Content truncated: {} total characters, showing first {}]\n",
            preview,
            char_count,
            FILE_PREVIEW_CHARS
        )
    }
}

/// Truncate tool result to prevent oversized API requests.
/// This is a safety net to avoid 400 errors from LLM providers.
pub fn truncate_tool_result(result: &str) -> String {
    let char_count = result.chars().count();
    if char_count <= MAX_TOOL_RESULT_CHARS {
        result.to_string()
    } else {
        let truncated: String = result.chars().take(MAX_TOOL_RESULT_CHARS).collect();
        format!(
            "{}\n\n... [Result truncated: {} total characters, showing first {}]",
            truncated,
            char_count,
            MAX_TOOL_RESULT_CHARS
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_ansi_colors() {
        // Red text: ESC[31mhello ESC[0m
        let input = "\x1b[31mhello\x1b[0m world";
        let output = remove_ansi_sequences(input);
        assert_eq!(output, "hello world");
    }

    #[test]
    fn test_remove_ansi_cursor_movement() {
        // ESC[2J clears screen, ESC[H moves cursor home
        let input = "\x1b[2J\x1b[Hcontent";
        let output = remove_ansi_sequences(input);
        assert_eq!(output, "content");
    }

    #[test]
    fn test_remove_ansi_complex() {
        // Bold + color + background
        let input = "\x1b[1;34;47mcolored\x1b[0m text";
        let output = remove_ansi_sequences(input);
        assert_eq!(output, "colored text");
    }

    #[test]
    fn test_remove_control_chars_basic() {
        // NULL, BEL, and other control characters
        let input = "hello\x00world\x07bell\x01\x02";
        let output = remove_control_chars(input);
        assert_eq!(output, "helloworldbell");
    }

    #[test]
    fn test_remove_control_chars_preserves_whitespace() {
        // Tab, newline, carriage return should be preserved
        let input = "line1\nline2\r\nline3\tindented";
        let output = remove_control_chars(input);
        assert_eq!(output, "line1\nline2\r\nline3\tindented");
    }

    #[test]
    fn test_sanitize_for_json_full() {
        // Combined ANSI and control characters
        let input = "\x1b[32msuccess\x1b[0m\n\x00\x07done";
        let output = sanitize_for_json(input);
        assert_eq!(output, "success\ndone");
    }

    #[test]
    fn test_sanitize_preserves_unicode() {
        // Chinese, emoji, Japanese
        let input = "你好世界 🐈 日本語";
        let output = sanitize_for_json(input);
        assert_eq!(output, "你好世界 🐈 日本語");
    }

    #[test]
    fn test_sanitize_empty_string() {
        assert_eq!(sanitize_for_json(""), "");
    }

    #[test]
    fn test_sanitize_only_control_chars() {
        let input = "\x00\x01\x02\x07\x1b";
        let output = sanitize_for_json(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_remove_ansi_osc_sequences() {
        // OSC title set sequence: ESC ] 0 ; title BEL
        let input = "\x1b]0;Terminal Title\x07prompt$ ";
        let output = remove_ansi_sequences(input);
        assert_eq!(output, "prompt$ ");
    }

    #[test]
    fn test_truncate_string_no_truncation_needed() {
        let input = "hello world";
        let output = truncate_string(input, 100, "...[truncated]");
        assert_eq!(output, "hello world");
    }

    #[test]
    fn test_truncate_string_truncation_applied() {
        let input = "hello world";
        let output = truncate_string(input, 5, "...[truncated]");
        assert_eq!(output, "hello...[truncated]");
    }

    #[test]
    fn test_truncate_file_content_small_file() {
        let input = "small file content";
        let output = truncate_file_content(input);
        assert_eq!(output, "small file content");
        assert!(!output.contains("truncated"));
    }

    #[test]
    fn test_truncate_file_content_large_file() {
        // Create a string larger than MAX_FILE_CONTENT_CHARS
        let large_content: String = "x".repeat(MAX_FILE_CONTENT_CHARS + 1000);
        let output = truncate_file_content(&large_content);
        assert!(output.contains("truncated"));
        assert!(output.contains(&format!("{} total characters", large_content.len())));
    }

    #[test]
    fn test_truncate_tool_result_small_result() {
        let input = "small result";
        let output = truncate_tool_result(input);
        assert_eq!(output, "small result");
        assert!(!output.contains("truncated"));
    }

    #[test]
    fn test_truncate_tool_result_large_result() {
        // Create a string larger than MAX_TOOL_RESULT_CHARS
        let large_result: String = "y".repeat(MAX_TOOL_RESULT_CHARS + 5000);
        let output = truncate_tool_result(&large_result);
        assert!(output.contains("truncated"));
        assert!(output.contains(&format!("{} total characters", large_result.len())));
    }

    #[test]
    fn test_truncate_unicode_correctly() {
        // Test that truncation works correctly with multi-byte characters
        let input = "你好世界这是测试内容";
        let output = truncate_string(input, 3, "...");
        assert_eq!(output, "你好世...");
    }
}
