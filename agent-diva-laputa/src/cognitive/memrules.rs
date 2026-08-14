//! MEMRULES.MD — the cognitive governance rule handbook.
//!
//! Human-edit only: no tool or API write path exists for this file. It is
//! read at startup and consulted as a policy reference on memory write
//! paths; it is never injected into the system prompt or ContextView.
//! When the file is missing, the built-in default rulebook applies.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{LaputaError, Result};

/// Built-in fallback rulebook, adapted from garden ADR-0004 §2.2 to
/// agent-diva semantics:
/// - R1 maps "raw material/evidence" to typed MemoryRecord evidence_refs
///   chains (the legacy memory engine was removed by GMH-24); evidence-less
///   writes stay advisory (Wave 6 evidence_advisory).
/// - R4 reflects direct BML writes; evidence remains advisory.
/// - R6/R7 reserve the WORLD entry gate and no-wholesale-injection rules.
pub const DEFAULT_MEM_RULES_TEXT: &str = "---
version: 1
updated: 2026-08-07T00:00:00Z
---

# Memory Rules

## R1 — Evidence primacy
Typed memory records' evidence_refs chains are the primary source of
truth. Memory writes without evidence stay advisory.

## R2 — Claim distinction
Confirmed fact, observation, inference, and hypothesis are distinct
categories and must not be conflated.

## R3 — Contradiction handling
New contradictory evidence does not silently overwrite prior
understanding; conflicts surface for review.

## R4 — User authority
User-confirmed information outranks agent inference. Memory writes apply
directly to BML with revision checks; evidence remains advisory.

## R5 — Scope constraint
Scope, time, confidence, provenance, and visibility constrain how a
memory may be used.

## R6 — WORLD entry gate
Entry into WORLD requires action relevance and a bounded, reviewable
claim.

## R7 — No wholesale injection
WORLD is never copied wholesale into an agent context; only bounded,
scope-matched projections may be used.
";

/// A single parsed rule (`## R<n> — title` heading plus body text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemRule {
    pub id: String,
    pub title: String,
    pub text: String,
}

/// Parsed MEMRULES.MD content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemRules {
    /// Source file path; `None` when backed by the built-in default.
    pub path: Option<PathBuf>,
    pub version: String,
    pub raw: String,
    pub rules: Vec<MemRule>,
}

impl MemRules {
    /// Parse the built-in default rulebook.
    pub fn defaults() -> Self {
        let mut rules = Self {
            path: None,
            version: String::new(),
            raw: DEFAULT_MEM_RULES_TEXT.to_string(),
            rules: Vec::new(),
        };
        rules.parse();
        rules
    }

    /// Load from disk, falling back to [`MemRules::defaults`] when the file
    /// does not exist. Other I/O failures are surfaced as errors.
    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        match fs::read_to_string(path) {
            Ok(raw) => {
                let mut rules = Self {
                    path: Some(path.to_path_buf()),
                    version: String::new(),
                    raw,
                    rules: Vec::new(),
                };
                rules.parse();
                Ok(rules)
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Self::defaults()),
            Err(source) => Err(LaputaError::io(path, source)),
        }
    }

    /// Look up a rule by id (e.g. `"R1"`).
    pub fn rule(&self, id: &str) -> Option<&MemRule> {
        self.rules.iter().find(|rule| rule.id == id)
    }

    fn parse(&mut self) {
        let mut in_front_matter = false;
        let mut front_matter_done = false;
        let mut current: Option<MemRule> = None;

        for line in self.raw.lines() {
            if !front_matter_done && line.trim() == "---" {
                if !in_front_matter {
                    in_front_matter = true;
                    continue;
                }
                in_front_matter = false;
                front_matter_done = true;
                continue;
            }
            if in_front_matter {
                if let Some(value) = line.strip_prefix("version:") {
                    self.version = value.trim().to_string();
                }
                continue;
            }

            if let Some(heading) = line.strip_prefix("## ") {
                if let Some(rule) = current.take() {
                    self.rules.push(rule);
                }
                let (id, title) = parse_rule_heading(heading);
                current = Some(MemRule {
                    id,
                    title,
                    text: String::new(),
                });
                continue;
            }

            if let Some(rule) = current.as_mut() {
                if !line.trim().is_empty() {
                    if !rule.text.is_empty() {
                        rule.text.push('\n');
                    }
                    rule.text.push_str(line);
                }
            }
        }
        if let Some(rule) = current.take() {
            self.rules.push(rule);
        }
    }
}

fn parse_rule_heading(heading: &str) -> (String, String) {
    for separator in [" \u{2014} ", " - "] {
        if let Some((id, title)) = heading.split_once(separator) {
            return (id.trim().to_string(), title.trim().to_string());
        }
    }
    (String::new(), heading.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_parse_seven_rules_with_stable_ids() {
        let rules = MemRules::defaults();
        assert_eq!(rules.version, "1");
        let ids: Vec<&str> = rules.rules.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["R1", "R2", "R3", "R4", "R5", "R6", "R7"]);
        assert_eq!(rules.rule("R1").unwrap().title, "Evidence primacy");
        assert!(rules.rule("R1").unwrap().text.contains("evidence_refs"));
    }

    #[test]
    fn load_or_default_falls_back_when_file_missing() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("MEMRULES.MD");
        let rules = MemRules::load_or_default(&missing).unwrap();
        assert_eq!(rules.path, None);
        assert_eq!(rules, MemRules::defaults());
    }

    #[test]
    fn load_parses_human_edited_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("MEMRULES.MD");
        std::fs::write(
            &path,
            "---\nversion: 3\n---\n\n# Memory Rules\n\n\
             ## R1 - custom heading style\nfirst line\nsecond line\n\n\
             ## R9 — extra rule\nhand added\n",
        )
        .unwrap();

        let rules = MemRules::load_or_default(&path).unwrap();
        assert_eq!(rules.path.as_deref(), Some(path.as_path()));
        assert_eq!(rules.version, "3");
        assert_eq!(rules.rules.len(), 2);
        assert_eq!(rules.rules[0].id, "R1");
        assert_eq!(rules.rules[0].title, "custom heading style");
        assert_eq!(rules.rules[0].text, "first line\nsecond line");
        assert_eq!(rules.rules[1].id, "R9");
        assert_eq!(rules.rules[1].text, "hand added");
    }
}
