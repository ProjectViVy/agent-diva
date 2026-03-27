//! Rational diary extraction policy for Phase A memory writes.

use agent_diva_memory::{
    sync_diary_entry_to_sqlite, DiaryEntry, DiaryPartition, DiaryStore, FileDiaryStore,
    MemoryDomain, MemoryScope, MemorySourceRef,
};
use regex::Regex;
use std::path::Path;

const POSITIVE_KEYWORDS: &[&str] = &[
    "架构",
    "模块",
    "目录",
    "文档",
    "设计",
    "实现",
    "方案",
    "计划",
    "阶段",
    "下一步",
    "建议",
    "仓库",
    "工作区",
    "分析",
    "验证",
    "memory",
    "architecture",
    "module",
    "workspace",
    "document",
    "docs",
    "design",
    "implement",
    "implementation",
    "plan",
    "phase",
    "next step",
    "recommend",
];
const NEGATIVE_KEYWORDS: &[&str] = &[
    "你好",
    "谢谢",
    "晚安",
    "早上好",
    "哈哈",
    "抱歉",
    "hello",
    "thanks",
];

/// Minimal extractor for analysis-oriented diary entries.
#[derive(Debug, Default)]
pub struct RationalDiaryExtractor;

impl RationalDiaryExtractor {
    pub fn extract(&self, user_input: &str, assistant_output: &str) -> Option<DiaryEntry> {
        let trimmed = assistant_output.trim();
        if trimmed.chars().count() < 120 {
            return None;
        }

        let lower = format!("{}\n{}", user_input.to_lowercase(), trimmed.to_lowercase());
        let positive_hits = POSITIVE_KEYWORDS
            .iter()
            .filter(|keyword| lower.contains(&keyword.to_lowercase()))
            .count();
        let negative_hits = NEGATIVE_KEYWORDS
            .iter()
            .filter(|keyword| lower.contains(&keyword.to_lowercase()))
            .count();
        let source_refs = extract_source_refs(trimmed);
        let has_structure =
            trimmed.contains('\n') || trimmed.contains("1.") || trimmed.contains("- ");

        if positive_hits < 2
            || !has_structure
            || (negative_hits > positive_hits && source_refs.is_empty())
        {
            return None;
        }

        let mut entry = DiaryEntry::new(
            DiaryPartition::Rational,
            detect_domain(trimmed),
            MemoryScope::Workspace,
            derive_title(user_input),
            derive_summary(trimmed),
            trimmed.to_string(),
        );
        entry.source_refs = source_refs;
        entry.tags = derive_tags(trimmed);
        entry.observations =
            extract_bullets_after_heading(trimmed, &["观察", "发现", "observations"]);
        entry.confirmed = extract_bullets_after_heading(trimmed, &["确认", "已确认", "confirmed"]);
        entry.unknowns =
            extract_bullets_after_heading(trimmed, &["未知", "待确认", "风险", "unknown"]);
        entry.next_steps =
            extract_bullets_after_heading(trimmed, &["下一步", "建议", "next", "follow-up"]);
        if entry.observations.is_empty() {
            entry.observations = collect_generic_bullets(trimmed, 3);
        }
        if entry.next_steps.is_empty() {
            entry.next_steps = collect_next_steps(trimmed);
        }
        entry.confidence = ((positive_hits as f32) / 6.0).clamp(0.55, 0.95);
        Some(entry)
    }

    pub fn persist_if_relevant<P: AsRef<Path>>(
        &self,
        workspace: P,
        user_input: &str,
        assistant_output: &str,
    ) -> agent_diva_core::Result<bool> {
        let Some(entry) = self.extract(user_input, assistant_output) else {
            return Ok(false);
        };
        let workspace = workspace.as_ref();
        let store = FileDiaryStore::new(workspace);
        store.append_entry(&entry)?;
        sync_diary_entry_to_sqlite(workspace, &entry)?;
        Ok(true)
    }
}

fn detect_domain(content: &str) -> MemoryDomain {
    let lower = content.to_lowercase();
    if lower.contains("文档")
        || lower.contains("docs")
        || lower.contains("readme")
        || lower.contains("architecture")
    {
        MemoryDomain::Workspace
    } else if lower.contains("任务")
        || lower.contains("todo")
        || lower.contains("下一步")
        || lower.contains("plan")
    {
        MemoryDomain::Task
    } else {
        MemoryDomain::DiaryRational
    }
}

fn derive_title(user_input: &str) -> String {
    let title = user_input
        .lines()
        .next()
        .unwrap_or("Rational diary note")
        .trim();
    let title = title.trim_matches('#').trim();
    let truncated = title.chars().take(60).collect::<String>();
    if truncated.is_empty() {
        "Rational diary note".into()
    } else {
        truncated
    }
}

fn derive_summary(content: &str) -> String {
    for line in content.lines() {
        let line = line.trim().trim_start_matches('-').trim();
        if !line.is_empty() && !line.starts_with('#') {
            return line.chars().take(140).collect();
        }
    }
    content.chars().take(140).collect()
}

fn derive_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let lower = content.to_lowercase();
    for (needle, tag) in [
        ("架构", "architecture"),
        ("architecture", "architecture"),
        ("文档", "docs"),
        ("docs", "docs"),
        ("memory", "memory"),
        ("设计", "design"),
        ("plan", "plan"),
        ("下一步", "next-step"),
        ("workspace", "workspace"),
    ] {
        if lower.contains(needle) && !tags.iter().any(|existing| existing == tag) {
            tags.push(tag.to_string());
        }
    }
    tags
}

fn extract_source_refs(content: &str) -> Vec<MemorySourceRef> {
    let Some(regex) = Regex::new(r"`([^`\n]+(?:/[^`\n]+)+)`").ok() else {
        return Vec::new();
    };

    let mut refs = Vec::new();
    for capture in regex.captures_iter(content) {
        let Some(path_match) = capture.get(1) else {
            continue;
        };
        refs.push(MemorySourceRef {
            path: Some(path_match.as_str().to_string()),
            section: None,
            note: None,
        });
    }
    refs.truncate(8);
    refs
}

fn extract_bullets_after_heading(content: &str, headings: &[&str]) -> Vec<String> {
    let mut capture = false;
    let mut items = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        let heading_match = headings
            .iter()
            .any(|heading| lower.contains(&heading.to_lowercase()));
        if heading_match
            && (trimmed.starts_with('#') || trimmed.ends_with(':') || trimmed.ends_with('：'))
        {
            capture = true;
            continue;
        }
        if capture {
            if trimmed.starts_with("##") || trimmed.starts_with("###") {
                break;
            }
            if let Some(value) = bullet_value(trimmed) {
                items.push(value.to_string());
            }
        }
    }
    items
}

fn collect_generic_bullets(content: &str, limit: usize) -> Vec<String> {
    let mut items = Vec::new();
    for line in content.lines() {
        if let Some(value) = bullet_value(line.trim()) {
            items.push(value.to_string());
            if items.len() >= limit {
                break;
            }
        }
    }
    items
}

fn collect_next_steps(content: &str) -> Vec<String> {
    let mut steps = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("建议")
            || trimmed.contains("下一步")
            || trimmed.to_lowercase().contains("next")
        {
            let cleaned = trimmed
                .trim_start_matches('-')
                .trim_start_matches('*')
                .trim()
                .to_string();
            if !cleaned.is_empty() {
                steps.push(cleaned);
            }
        }
    }
    steps.truncate(3);
    steps
}

fn bullet_value(line: &str) -> Option<&str> {
    if line.starts_with("- ") || line.starts_with("* ") {
        Some(line[2..].trim())
    } else if line.len() > 3
        && line.as_bytes()[0].is_ascii_digit()
        && line.as_bytes()[1] == b'.'
        && line.as_bytes()[2] == b' '
    {
        Some(line[3..].trim())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_memory::MemoryStore;
    use tempfile::TempDir;

    #[test]
    fn test_extracts_analysis_entry() {
        let extractor = RationalDiaryExtractor;
        let assistant_output = r#"
## 架构分析
- 观察：`agent-diva-core/src/memory/mod.rs` 当前只暴露 MEMORY.md 相关能力。
- 已确认：`agent-diva-agent/src/context.rs` 只注入长期记忆，不注入 diary。
- 下一步：先增加 diary store，再挂接提取策略。
- 建议：保留 MEMORY.md 兼容路径。
"#;

        let entry = extractor
            .extract("请分析当前记忆架构并给出下一步方案", assistant_output)
            .unwrap();
        assert_eq!(entry.partition, DiaryPartition::Rational);
        assert_eq!(entry.scope, MemoryScope::Workspace);
        assert!(!entry.source_refs.is_empty());
        assert!(!entry.next_steps.is_empty());
    }

    #[test]
    fn test_skips_casual_reply() {
        let extractor = RationalDiaryExtractor;
        let assistant_output = "你好，今天过得怎么样？谢谢你。";
        assert!(extractor.extract("打个招呼", assistant_output).is_none());
    }

    #[test]
    fn test_persist_if_relevant() {
        let temp_dir = TempDir::new().unwrap();
        let extractor = RationalDiaryExtractor;
        let assistant_output = r#"
## 实现方案
- 观察：`agent-diva-core/src/memory/manager.rs` 目前没有 diary 路径 helper。
- 建议：增加 `memory/diary/rational/YYYY-MM-DD.md` 落盘。
- 下一步：在 loop turn 中挂接提取器。
- 设计：保持 MEMORY.md 行为不变。
"#;

        let persisted = extractor
            .persist_if_relevant(temp_dir.path(), "请给出记忆实现方案", assistant_output)
            .unwrap();
        assert!(persisted);

        let store = FileDiaryStore::new(temp_dir.path());
        let days = store.list_days(DiaryPartition::Rational).unwrap();
        assert_eq!(days.len(), 1);
        let entries = store.load_day(&days[0], DiaryPartition::Rational).unwrap();
        assert_eq!(entries.len(), 1);

        let memory_store = agent_diva_memory::SqliteMemoryStore::new(temp_dir.path()).unwrap();
        let records = memory_store.list_records().unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0].id.starts_with("diary:"));
    }
}
