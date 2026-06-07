//! Summary quality validation for context compaction.
//!
//! Provides [`validate_summary`] which scores a generated summary against
//! the source messages on three axes: length, keyword coverage, and semantic
//! completeness. The composite [`QualityReport::score`] is used by
//! [`ContextCompactor`](super::ContextCompactor) to decide whether to retry.

use agent_diva_core::session::ChatMessage;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// QualityReport
// ---------------------------------------------------------------------------

/// Result of a summary quality evaluation.
#[derive(Debug, Clone)]
pub struct QualityReport {
    /// Composite quality score (0.0–1.0), weighted average of the three axes.
    pub score: f64,
    /// Length sub-score (0.0–1.0).
    pub length_score: f64,
    /// Keyword coverage sub-score (0.0–1.0).
    pub keyword_score: f64,
    /// Semantic completeness sub-score (0.0–1.0).
    pub completeness_score: f64,
    /// Human-readable issues detected.
    pub issues: Vec<String>,
}

// ---------------------------------------------------------------------------
// Keyword extraction
// ---------------------------------------------------------------------------

/// English identifier regex: `[a-zA-Z_][a-zA-Z0-9_]{2,}` (length >= 3).
static RE_EN_IDENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"[a-zA-Z_][a-zA-Z0-9_]{2,}").unwrap());

/// File path token: contains `/` or `.` (e.g. `src/main.rs`, `Cargo.toml`).
static RE_PATH: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[a-zA-Z0-9_./\\-]+\.[a-zA-Z0-9]{1,10}").unwrap());

/// Stop-words to exclude from keyword sets.
static STOP_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // English
        "the", "and", "for", "are", "but", "not", "you", "all", "can", "had", "her", "was", "one",
        "our", "out", "has", "his", "how", "its", "may", "new", "now", "old", "see", "way", "who",
        "did", "get", "let", "say", "she", "too", "use", // Chinese
        "的", "是", "在", "了", "有", "和", "我", "你", "他", "她", "它", "们", "这", "那", "就",
        "也", "都", "要", "会", "能", "对", "说", "到", "可以", "没有", "什么", "这个", "那个",
        "一个", "如果", "因为", "所以", "但是", "然后", "或者", "已经", "还是", "不是", "而且",
        "虽然", "这样", "那样", "非常", "一些", "一下", "自己", "知道", "请", "把", "被", "从",
        "向", "让", "给", "用",
    ]
    .iter()
    .copied()
    .collect()
});

/// Check if a character is a CJK (Chinese/Japanese/Korean) character.
/// Covers the main CJK Unified Ideographs block (U+4E00–U+9FFF).
fn is_cjk(ch: char) -> bool {
    matches!(ch, '\u{4e00}'..='\u{9fff}')
}

/// Extract keywords from a text block.
///
/// Returns a set of lowercased tokens (English idents, Chinese words, paths).
fn extract_keywords(text: &str) -> HashSet<String> {
    let mut keywords = HashSet::new();

    // English identifiers
    for m in RE_EN_IDENT.find_iter(text) {
        let word = m.as_str().to_lowercase();
        if !STOP_WORDS.contains(word.as_str()) {
            keywords.insert(word);
        }
    }

    // Chinese words — manual scanner for consecutive CJK characters
    let mut current_word = String::new();
    for ch in text.chars() {
        if is_cjk(ch) {
            current_word.push(ch);
        } else {
            if current_word.len() >= 2 && !STOP_WORDS.contains(current_word.as_str()) {
                keywords.insert(current_word.clone());
            }
            current_word.clear();
        }
    }
    // Don't forget the last word
    if current_word.len() >= 2 && !STOP_WORDS.contains(current_word.as_str()) {
        keywords.insert(current_word);
    }

    // File paths
    for m in RE_PATH.find_iter(text) {
        keywords.insert(m.as_str().to_lowercase());
    }

    keywords
}

// ---------------------------------------------------------------------------
// Sub-scorers
// ---------------------------------------------------------------------------

/// Score summary length. Full marks at 50+ chars; 0 at 0 chars.
fn score_length(summary: &str) -> (f64, Option<String>) {
    let len = summary.chars().count();
    if len >= 200 {
        (1.0, None)
    } else if len >= 50 {
        // Linear scale from 0.6 (at 50) to 1.0 (at 200)
        let s = 0.6 + 0.4 * ((len - 50) as f64 / 150.0);
        (s, None)
    } else if len > 0 {
        let s = (len as f64 / 50.0) * 0.6;
        (s, Some(format!("摘要过短（{} 字符，至少需要 50）", len)))
    } else {
        (0.0, Some("摘要为空".to_string()))
    }
}

/// Score keyword coverage: how many source keywords appear in the summary.
fn score_keywords(summary: &str, source_messages: &[ChatMessage]) -> (f64, Option<String>) {
    let source_text: String = source_messages.iter().map(|m| m.content.as_str()).collect();
    let source_kw = extract_keywords(&source_text);

    if source_kw.is_empty() {
        // No meaningful keywords in source — give full marks (nothing to cover)
        return (1.0, None);
    }

    let summary_lower = summary.to_lowercase();
    let matched = source_kw
        .iter()
        .filter(|kw| summary_lower.contains(kw.as_str()))
        .count();

    let ratio = matched as f64 / source_kw.len() as f64;

    if ratio >= 0.3 {
        (ratio.min(1.0), None)
    } else {
        (
            ratio,
            Some(format!(
                "关键词覆盖率过低（{:.0}%，至少需要 30%）",
                ratio * 100.0
            )),
        )
    }
}

/// Score semantic completeness: does the summary contain at least one
/// sentence-ending punctuation mark?
fn score_completeness(summary: &str) -> (f64, Option<String>) {
    // Empty summary has no completeness at all
    if summary.is_empty() {
        return (0.0, Some("摘要为空".to_string()));
    }

    // Check for sentence-ending punctuation (Chinese + English)
    let has_sentence = summary.contains('。')
        || summary.contains('？')
        || summary.contains('！')
        || summary.contains('?')
        || summary.contains('!')
        || summary.contains('.');

    if has_sentence {
        // Bonus: check for reasonable sentence count
        let sentence_count = summary.matches('。').count()
            + summary.matches('？').count()
            + summary.matches('！').count()
            + summary.matches(". ").count()
            + summary.matches("? ").count()
            + summary.matches("! ").count();

        if sentence_count >= 3 {
            (1.0, None)
        } else if sentence_count >= 1 {
            (0.8, None)
        } else {
            // Has a period but no clear sentence boundaries
            (0.6, None)
        }
    } else {
        (
            0.2,
            Some("摘要缺少完整句子（无句号/问号/感叹号）".to_string()),
        )
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate summary quality against the source messages.
///
/// Returns a [`QualityReport`] with a composite score (0.0–1.0) and
/// per-axis diagnostics. The weights are:
/// - Length: 20%
/// - Keyword coverage: 40%
/// - Semantic completeness: 40%
pub fn validate_summary(summary: &str, source_messages: &[ChatMessage]) -> QualityReport {
    let (length_score, length_issue) = score_length(summary);
    let (keyword_score, keyword_issue) = score_keywords(summary, source_messages);
    let (completeness_score, completeness_issue) = score_completeness(summary);

    let mut issues = Vec::new();
    if let Some(issue) = length_issue {
        issues.push(issue);
    }
    if let Some(issue) = keyword_issue {
        issues.push(issue);
    }
    if let Some(issue) = completeness_issue {
        issues.push(issue);
    }

    // Weighted average: length 20%, keywords 40%, completeness 40%
    let score = length_score * 0.2 + keyword_score * 0.4 + completeness_score * 0.4;

    QualityReport {
        score,
        length_score,
        keyword_score,
        completeness_score,
        issues,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_msg(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            reasoning_content: None,
            thinking_blocks: None,
        }
    }

    // -- extract_keywords --

    #[test]
    fn test_extract_keywords_english() {
        let kw = extract_keywords("fn main() { println!(\"hello\"); }");
        assert!(kw.contains("main"), "should contain 'main'");
        assert!(kw.contains("println"), "should contain 'println'");
        // "fn" is only 2 chars, should be excluded
        assert!(!kw.contains("fn"));
    }

    #[test]
    fn test_extract_keywords_chinese() {
        let text = "用户请求修改配置文件，助手已执行命令";
        let kw = extract_keywords(text);
        // The scanner groups consecutive CJK chars — comma breaks the run
        // So we get two keywords: "用户请求修改配置文件" and "助手已执行命令"
        let kw_concat: Vec<&str> = kw.iter().map(|s| s.as_str()).collect();
        assert!(
            kw_concat.iter().any(|k| k.contains("用户")),
            "should find keyword containing '用户', got {:?}",
            kw
        );
        assert!(
            kw_concat.iter().any(|k| k.contains("请求")),
            "should find keyword containing '请求'"
        );
        assert!(
            kw_concat.iter().any(|k| k.contains("配置")),
            "should find keyword containing '配置'"
        );
        assert!(!kw.contains("了"), "stop-word '了' should be filtered");
    }

    #[test]
    fn test_extract_keywords_paths() {
        let kw = extract_keywords("编辑了 src/main.rs 和 Cargo.toml");
        assert!(kw.contains("src/main.rs"));
        assert!(kw.contains("cargo.toml")); // lowercased
    }

    // -- score_length --

    #[test]
    fn test_length_empty() {
        let (score, issue) = score_length("");
        assert_eq!(score, 0.0);
        assert!(issue.is_some());
    }

    #[test]
    fn test_length_short() {
        let (score, issue) = score_length("太短了");
        assert!(score < 0.6);
        assert!(issue.is_some());
    }

    #[test]
    fn test_length_adequate() {
        let text = "a".repeat(100);
        let (score, issue) = score_length(&text);
        assert!(score >= 0.6);
        assert!(issue.is_none());
    }

    #[test]
    fn test_length_full() {
        let text = "a".repeat(250);
        let (score, issue) = score_length(&text);
        assert_eq!(score, 1.0);
        assert!(issue.is_none());
    }

    // -- score_keywords --

    #[test]
    fn test_keyword_coverage_good() {
        let sources = vec![
            make_msg("user", "请帮我修改 src/main.rs 文件中的配置"),
            make_msg("assistant", "已修改配置文件，添加了新的 config 选项"),
        ];
        let summary = "用户请求修改 src/main.rs 配置文件，助手添加了 config 选项。";
        let (score, issue) = score_keywords(summary, &sources);
        assert!(score >= 0.3, "score should be >= 0.3, got {}", score);
        assert!(issue.is_none());
    }

    #[test]
    fn test_keyword_coverage_poor() {
        let sources = vec![
            make_msg("user", "请帮我修改 src/main.rs 文件中的配置"),
            make_msg("assistant", "已修改配置文件，添加了新的 config 选项"),
        ];
        let summary = "对话结束了。";
        let (score, issue) = score_keywords(summary, &sources);
        assert!(score < 0.3, "score should be < 0.3, got {}", score);
        assert!(issue.is_some());
    }

    #[test]
    fn test_keyword_empty_source() {
        let sources = vec![make_msg("user", "")];
        let (score, _) = score_keywords("任何摘要", &sources);
        assert_eq!(score, 1.0); // no keywords to cover
    }

    // -- score_completeness --

    #[test]
    fn test_completeness_with_sentences() {
        let (score, issue) = score_completeness("用户请求帮助。助手已回复。对话结束。");
        assert!(score >= 0.8);
        assert!(issue.is_none());
    }

    #[test]
    fn test_completeness_no_sentence() {
        let (score, issue) = score_completeness("一些没有标点的内容");
        assert!(score < 0.5);
        assert!(issue.is_some());
    }

    #[test]
    fn test_completeness_english_period() {
        let (score, issue) = score_completeness("The user asked for help. The assistant replied.");
        assert!(score >= 0.8);
        assert!(issue.is_none());
    }

    // -- validate_summary (composite) --

    #[test]
    fn test_validate_good_summary() {
        let sources = vec![
            make_msg("user", "请帮我修改 src/main.rs 文件中的配置"),
            make_msg(
                "assistant",
                "已修改配置文件，添加了新的 config 选项。编译通过。",
            ),
        ];
        let summary =
            "用户请求修改 src/main.rs 配置文件，助手成功添加了新的 config 选项，项目编译顺利通过。";
        let report = validate_summary(summary, &sources);
        assert!(
            report.score >= 0.6,
            "good summary should score >= 0.6, got {:.2}",
            report.score
        );
        assert!(
            report.issues.is_empty(),
            "good summary should have no issues, got {:?}",
            report.issues
        );
    }

    #[test]
    fn test_validate_bad_summary() {
        let sources = vec![
            make_msg("user", "请帮我修改 src/main.rs 文件中的配置"),
            make_msg(
                "assistant",
                "已修改配置文件，添加了新的 config 选项。编译通过。",
            ),
        ];
        let summary = "嗯";
        let report = validate_summary(summary, &sources);
        assert!(
            report.score < 0.6,
            "bad summary should score < 0.6, got {:.2}",
            report.score
        );
        assert!(!report.issues.is_empty(), "bad summary should have issues");
    }

    #[test]
    fn test_validate_empty_summary() {
        let sources = vec![make_msg("user", "测试内容")];
        let report = validate_summary("", &sources);
        assert_eq!(report.score, 0.0);
        assert!(!report.issues.is_empty());
    }

    #[test]
    fn test_score_range() {
        // Score should always be in [0.0, 1.0]
        let sources = vec![make_msg("user", &"x".repeat(5000))];
        let report = validate_summary("短", &sources);
        assert!(report.score >= 0.0 && report.score <= 1.0);
    }
}
