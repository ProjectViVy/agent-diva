//! Utility functions and helpers

use std::path::Path;

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> std::path::PathBuf {
    let path = path.as_ref();
    if !path.exists() {
        let _ = std::fs::create_dir_all(path);
    }
    path.to_path_buf()
}

/// Create a safe filename from a string
pub fn safe_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '_',
        })
        .collect()
}

/// Truncate a string to a maximum byte length, ensuring valid UTF-8 boundaries
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len.saturating_sub(3);
        while !s.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        format!("{}...", &s[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_filename() {
        assert_eq!(safe_filename("hello world"), "hello_world");
        assert_eq!(safe_filename("test/file:name"), "test_file_name");
        assert_eq!(safe_filename("normal-name.txt"), "normal-name.txt");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("test", 3), "...");
    }
}
