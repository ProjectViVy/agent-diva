//! Derive stable structured memory records from diary entries.

use crate::types::{
    DiaryEntry, DiaryPartition, MemoryDomain, MemoryRecord, MemoryScope, MemorySourceRef,
};
use chrono::Local;
use std::collections::BTreeSet;

const RELATIONSHIP_KEYWORDS: &[&str] = &[
    "用户",
    "user",
    "偏好",
    "prefer",
    "喜欢",
    "不喜欢",
    "协作",
    "沟通",
    "约束",
    "要求",
    "避免",
    "回复",
    "中文",
    "简洁",
    "直接",
    "前缀",
    "commit",
    "提交",
];
const SELF_MODEL_KEYWORDS: &[&str] = &[
    "我是",
    "我会",
    "我应该",
    "我不",
    "这个 agent",
    "该 agent",
    "this agent",
    "assistant",
    "助手",
    "能力",
    "局限",
    "边界",
    "工作方式",
    "角色",
    "擅长",
];
const SOUL_RULE_KEYWORDS: &[&str] = &[
    "必须", "始终", "优先", "不要", "禁止", "must", "always", "never",
];
const SOUL_IDENTITY_KEYWORDS: &[&str] = &[
    "风格", "语气", "身份", "人格", "透明", "规则", "原则", "中文", "前缀", "沟通", "soul",
    "identity",
];

pub fn derive_structured_memory_records(entry: &DiaryEntry) -> Vec<MemoryRecord> {
    if !matches!(entry.partition, DiaryPartition::Rational) {
        return Vec::new();
    }

    let signals = collect_candidate_signals(entry);
    let mut records = Vec::new();

    if let Some(record) = build_relationship_record(entry, &signals) {
        records.push(record);
    }
    if let Some(record) = build_self_model_record(entry, &signals) {
        records.push(record);
    }
    if let Some(record) = build_soul_signal_record(entry, &signals) {
        records.push(record);
    }

    records
}

fn build_relationship_record(entry: &DiaryEntry, signals: &[String]) -> Option<MemoryRecord> {
    let matched = matched_signals(signals, |line| {
        contains_any_keyword(line, RELATIONSHIP_KEYWORDS)
    });
    if matched.is_empty() {
        return None;
    }

    Some(build_record(
        entry,
        MemoryDomain::Relationship,
        MemoryScope::User,
        "Relationship signal",
        &matched,
        &["relationship", "user-preference", "derived"],
    ))
}

fn build_self_model_record(entry: &DiaryEntry, signals: &[String]) -> Option<MemoryRecord> {
    let matched = matched_signals(signals, |line| {
        contains_any_keyword(line, SELF_MODEL_KEYWORDS)
    });
    if matched.is_empty() {
        return None;
    }

    Some(build_record(
        entry,
        MemoryDomain::SelfModel,
        MemoryScope::Workspace,
        "Self-model signal",
        &matched,
        &["self-model", "agent-working-style", "derived"],
    ))
}

fn build_soul_signal_record(entry: &DiaryEntry, signals: &[String]) -> Option<MemoryRecord> {
    let matched = matched_signals(signals, |line| {
        contains_any_keyword(line, SOUL_RULE_KEYWORDS)
            && contains_any_keyword(line, SOUL_IDENTITY_KEYWORDS)
    });
    if matched.is_empty() {
        return None;
    }

    Some(build_record(
        entry,
        MemoryDomain::SoulSignal,
        MemoryScope::Workspace,
        "Soul signal",
        &matched,
        &["soul-signal", "identity-signal", "derived"],
    ))
}

fn build_record(
    entry: &DiaryEntry,
    domain: MemoryDomain,
    scope: MemoryScope,
    title_prefix: &str,
    matched: &[String],
    extra_tags: &[&str],
) -> MemoryRecord {
    let summary = matched
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join("；");
    let mut tags = entry.tags.clone();
    for tag in extra_tags {
        if !tags.iter().any(|existing| existing == tag) {
            tags.push((*tag).to_string());
        }
    }

    MemoryRecord {
        id: format!("derived:{}:{}", domain_slug(&domain), entry.id),
        timestamp: entry.timestamp,
        domain,
        scope,
        title: format!("{title_prefix}: {}", entry.title.trim()),
        summary,
        content: render_derived_content(entry, matched),
        tags,
        source_refs: derived_source_refs(entry),
        confidence: (entry.confidence + 0.05).clamp(0.60, 0.98),
    }
}

fn render_derived_content(entry: &DiaryEntry, matched: &[String]) -> String {
    let mut lines = vec![
        format!("Derived from rational diary entry: {}", entry.title.trim()),
        format!("Diary summary: {}", entry.summary.trim()),
        String::new(),
        "Stable signals:".to_string(),
    ];

    for item in matched {
        lines.push(format!("- {item}"));
    }

    lines.join("\n")
}

fn derived_source_refs(entry: &DiaryEntry) -> Vec<MemorySourceRef> {
    let mut refs = vec![MemorySourceRef {
        path: Some(diary_source_path(entry)),
        section: Some(entry.title.clone()),
        note: Some(format!("derived from diary entry {}", entry.id)),
    }];
    refs.extend(entry.source_refs.clone());
    refs
}

fn diary_source_path(entry: &DiaryEntry) -> String {
    let partition = match entry.partition {
        DiaryPartition::Rational => "rational",
        DiaryPartition::Emotional => "emotional",
    };
    let date = entry.timestamp.with_timezone(&Local).format("%Y-%m-%d");
    format!("memory/diary/{partition}/{date}.md")
}

fn matched_signals<F>(signals: &[String], mut predicate: F) -> Vec<String>
where
    F: FnMut(&str) -> bool,
{
    let mut matched = Vec::new();
    for line in signals {
        if predicate(line) {
            matched.push(line.clone());
        }
    }
    matched.truncate(3);
    matched
}

fn collect_candidate_signals(entry: &DiaryEntry) -> Vec<String> {
    let mut items = BTreeSet::new();
    for line in [&entry.summary, &entry.title] {
        let cleaned = normalize_signal(line);
        if !cleaned.is_empty() {
            items.insert(cleaned);
        }
    }

    for section in [
        &entry.observations,
        &entry.confirmed,
        &entry.unknowns,
        &entry.next_steps,
    ] {
        for value in section {
            let cleaned = normalize_signal(value);
            if !cleaned.is_empty() {
                items.insert(cleaned);
            }
        }
    }

    for line in entry.body.lines() {
        let cleaned = normalize_signal(line);
        if !cleaned.is_empty() {
            items.insert(cleaned);
        }
    }

    items.into_iter().collect()
}

fn normalize_signal(line: &str) -> String {
    let mut cleaned = line
        .trim()
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim_matches('`')
        .replace('`', "")
        .replace("观察：", "")
        .replace("已确认：", "")
        .replace("确认：", "")
        .replace("下一步：", "")
        .replace("建议：", "")
        .replace("未知：", "")
        .replace("风险：", "")
        .trim()
        .to_string();

    if cleaned.len() > 160 {
        cleaned = cleaned.chars().take(160).collect();
    }
    if cleaned.starts_with('#') || cleaned.is_empty() {
        return String::new();
    }
    cleaned
}

fn contains_any_keyword(line: &str, keywords: &[&str]) -> bool {
    let lower = line.to_lowercase();
    keywords
        .iter()
        .any(|keyword| lower.contains(&keyword.to_lowercase()))
}

fn domain_slug(domain: &MemoryDomain) -> &'static str {
    match domain {
        MemoryDomain::Relationship => "relationship",
        MemoryDomain::SelfModel => "self_model",
        MemoryDomain::SoulSignal => "soul_signal",
        MemoryDomain::Fact => "fact",
        MemoryDomain::Event => "event",
        MemoryDomain::Task => "task",
        MemoryDomain::Workspace => "workspace",
        MemoryDomain::DiaryRational => "diary_rational",
        MemoryDomain::DiaryEmotional => "diary_emotional",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn sample_entry() -> DiaryEntry {
        let mut entry = DiaryEntry::new(
            DiaryPartition::Rational,
            MemoryDomain::Workspace,
            MemoryScope::Workspace,
            "记忆治理方案",
            "用户希望回复使用中文并保持简洁。",
            r#"
## 结论
- 已确认：用户偏好中文回复，尽量保持简洁直接。
- 已确认：所有回复必须以前缀 [I strictly follow the rules] 开头。
- 已确认：这个 agent 应该先说明动作再改文件，不要自作主张提交代码。
"#,
        );
        entry.timestamp = DateTime::parse_from_rfc3339("2026-03-27T08:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        entry.confirmed = vec![
            "用户偏好中文回复，尽量保持简洁直接。".into(),
            "所有回复必须以前缀 [I strictly follow the rules] 开头。".into(),
            "这个 agent 应该先说明动作再改文件，不要自作主张提交代码。".into(),
        ];
        entry
    }

    #[test]
    fn derive_structured_records_from_rational_diary() {
        let records = derive_structured_memory_records(&sample_entry());
        assert_eq!(records.len(), 3);
        assert!(records
            .iter()
            .any(|record| record.domain == MemoryDomain::Relationship));
        assert!(records
            .iter()
            .any(|record| record.domain == MemoryDomain::SelfModel));
        assert!(records
            .iter()
            .any(|record| record.domain == MemoryDomain::SoulSignal));
    }

    #[test]
    fn crate_name_does_not_trigger_self_model() {
        let mut entry = DiaryEntry::new(
            DiaryPartition::Rational,
            MemoryDomain::Workspace,
            MemoryScope::Workspace,
            "Architecture note",
            "Mapped the memory split",
            "The agent-diva-memory crate now owns diary storage.",
        );
        entry.timestamp = DateTime::parse_from_rfc3339("2026-03-27T08:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let records = derive_structured_memory_records(&entry);
        assert!(records.is_empty());
    }

    #[test]
    fn emotional_diary_does_not_create_derived_records() {
        let mut entry = sample_entry();
        entry.partition = DiaryPartition::Emotional;
        let records = derive_structured_memory_records(&entry);
        assert!(records.is_empty());
    }
}
