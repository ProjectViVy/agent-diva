//! Error context utilities for debugging
//!
//! Provides utilities to capture and log problematic content when errors occur.

use std::collections::HashMap;

/// Maximum length of content to include in error context
const MAX_CONTEXT_LENGTH: usize = 500;

/// Maximum length for individual problematic token/character
const MAX_TOKEN_LENGTH: usize = 100;

/// Context information captured when an error occurs
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The operation that failed
    pub operation: String,
    /// Error message
    pub error_message: String,
    /// The problematic content that caused the error
    pub problematic_content: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(operation: impl Into<String>, error_message: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            error_message: error_message.into(),
            problematic_content: None,
            metadata: HashMap::new(),
        }
    }

    /// Add problematic content
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.problematic_content = Some(truncate_content(&content.into(), MAX_CONTEXT_LENGTH));
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Format the context as a human-readable string
    pub fn to_detailed_string(&self) -> String {
        let mut parts = vec![format!("[{}] {}", self.operation, self.error_message)];

        if let Some(ref content) = self.problematic_content {
            parts.push(format!("Problematic content: {}", content));
        }

        for (key, value) in &self.metadata {
            parts.push(format!("{}: {}", key, value));
        }

        parts.join("\n  ")
    }
}

/// Truncate content to a maximum length while preserving UTF-8 boundaries
fn truncate_content(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len.saturating_sub(3);
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

/// Find and highlight problematic characters in content
/// Returns a description of problematic characters found
pub fn find_problematic_chars(content: &str) -> Vec<String> {
    let mut problems = Vec::new();

    for (i, c) in content.char_indices() {
        let cp = c as u32;

        // Check for control characters (except allowed whitespace)
        if cp < 0x20 && cp != 0x09 && cp != 0x0A && cp != 0x0D {
            let context = get_char_context(content, i, MAX_TOKEN_LENGTH);
            problems.push(format!(
                "Control char U+{:04X} at position {}: '{}'",
                cp,
                i,
                escape_for_display(&context)
            ));
        }

        // Check for DEL character
        if cp == 0x7F {
            let context = get_char_context(content, i, MAX_TOKEN_LENGTH);
            problems.push(format!(
                "DEL char (U+007F) at position {}: '{}'",
                i,
                escape_for_display(&context)
            ));
        }

        // Check for invalid UTF-8 sequences (though Rust handles this)
        if c == '\u{FFFD}' {
            let context = get_char_context(content, i, MAX_TOKEN_LENGTH);
            problems.push(format!(
                "Replacement char (U+FFFD) at position {}: '{}'",
                i,
                escape_for_display(&context)
            ));
        }
    }

    problems
}

/// Get context around a character position
fn get_char_context(content: &str, pos: usize, max_len: usize) -> String {
    let start = pos.saturating_sub(max_len / 2);
    let end = (pos + max_len / 2).min(content.len());

    // Ensure valid UTF-8 boundaries
    let start = find_char_boundary(content, start);
    let end = find_char_boundary_back(content, end);

    content[start..end].to_string()
}

/// Find the nearest valid UTF-8 char boundary at or after the given position
fn find_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        return s.len();
    }
    let mut p = pos;
    while p < s.len() && !s.is_char_boundary(p) {
        p += 1;
    }
    p
}

/// Find the nearest valid UTF-8 char boundary at or before the given position
fn find_char_boundary_back(s: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut p = pos;
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

/// Escape a string for display in logs
fn escape_for_display(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            c if c.is_control() => format!("\\u{:04X}", c as u32),
            c => c.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_basic() {
        let ctx = ErrorContext::new("test_op", "test error");
        assert_eq!(ctx.operation, "test_op");
        assert_eq!(ctx.error_message, "test error");
        assert!(ctx.problematic_content.is_none());
    }

    #[test]
    fn test_error_context_with_content() {
        let ctx = ErrorContext::new("test", "error").with_content("problematic content");
        assert_eq!(
            ctx.problematic_content,
            Some("problematic content".to_string())
        );
    }

    #[test]
    fn test_error_context_truncation() {
        let long_content = "x".repeat(1000);
        let ctx = ErrorContext::new("test", "error").with_content(long_content.clone());
        assert!(ctx.problematic_content.as_ref().unwrap().len() < long_content.len());
        assert!(ctx.problematic_content.as_ref().unwrap().ends_with("..."));
    }

    #[test]
    fn test_find_problematic_chars_control() {
        let content = "hello\x01world"; // Contains SOH control char
        let problems = find_problematic_chars(content);
        assert!(!problems.is_empty());
        assert!(problems[0].contains("U+0001"));
        assert!(problems[0].contains("position 5"));
    }

    #[test]
    fn test_find_problematic_chars_clean() {
        let content = "hello world\nthis is normal";
        let problems = find_problematic_chars(content);
        assert!(problems.is_empty());
    }

    #[test]
    fn test_escape_for_display() {
        assert_eq!(escape_for_display("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_for_display("tab\there"), "tab\\there");
        assert_eq!(escape_for_display("\x01"), "\\u0001");
    }
}
