use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
struct ForbiddenPattern {
    needle: &'static str,
}

const FORBIDDEN_PATTERNS: &[ForbiddenPattern] = &[
    ForbiddenPattern {
        needle: ".laputa/sections",
    },
    ForbiddenPattern {
        needle: ".laputa/changelog",
    },
    ForbiddenPattern {
        needle: ".laputa/audit",
    },
    ForbiddenPattern {
        needle: ".laputa/rollback",
    },
    ForbiddenPattern {
        needle: "MEMORY.md",
    },
    ForbiddenPattern {
        needle: "IDENTITY.md",
    },
    ForbiddenPattern { needle: "SOUL.md" },
];

const STATIC_SCAN_ROOTS: &[&str] = &[
    "agent-diva-agent/src",
    "agent-diva-autodream/src",
    "agent-diva-manager/src",
    "agent-diva-cli/src",
];

const ALLOWLIST: &[&str] = &[
    "agent-diva-agent/src/context.rs",
    "agent-diva-agent/src/subagent.rs",
];

const WRITE_CALL_MARKERS: &[&str] = &[
    "fs::write(",
    "std::fs::write(",
    "tokio::fs::write(",
    "File::create(",
    "OpenOptions::new()",
    "atomic_write(",
    "atomic_write_json(",
];

#[test]
fn governance_direct_write_guard_only_allows_laputa_owned_authority_paths() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let mut violations = Vec::new();

    for root in STATIC_SCAN_ROOTS {
        collect_violations(&repo_root.join(root), &repo_root, &mut violations);
    }

    assert!(
        violations.is_empty(),
        "governance authority path references must stay behind Laputa boundaries:\n{}",
        violations.join("\n")
    );
}

fn collect_violations(scan_root: &Path, repo_root: &Path, violations: &mut Vec<String>) {
    if !scan_root.exists() {
        return;
    }

    for entry in std::fs::read_dir(scan_root).expect("scan dir") {
        let entry = entry.expect("scan entry");
        let path = entry.path();
        if path.is_dir() {
            collect_violations(&path, repo_root, violations);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let relative = path
            .strip_prefix(repo_root)
            .expect("path under repo root")
            .to_string_lossy()
            .replace('\\', "/");

        if ALLOWLIST.iter().any(|allowed| *allowed == relative) {
            continue;
        }

        let content = std::fs::read_to_string(&path).expect("read source file");
        let mut in_test_module = false;
        let mut test_brace_depth = 0usize;
        let mut pending_test_module = false;
        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed == "#[cfg(test)]" {
                pending_test_module = true;
            } else if pending_test_module && trimmed.starts_with("mod tests") {
                in_test_module = true;
                test_brace_depth = brace_delta(line);
                pending_test_module = false;
                continue;
            } else if pending_test_module && !trimmed.is_empty() {
                pending_test_module = false;
            }

            if in_test_module {
                test_brace_depth = test_brace_depth.saturating_add(brace_delta(line));
                test_brace_depth = test_brace_depth.saturating_sub(close_brace_count(line));
                if test_brace_depth == 0 {
                    in_test_module = false;
                }
                continue;
            }

            if line.trim_start().starts_with("//") {
                continue;
            }

            if !WRITE_CALL_MARKERS
                .iter()
                .any(|marker| line.contains(marker))
            {
                continue;
            }

            for pattern in FORBIDDEN_PATTERNS {
                if line.contains(pattern.needle) {
                    violations.push(format!(
                        "{}:{} references forbidden authority path `{}`",
                        relative,
                        line_idx + 1,
                        pattern.needle
                    ));
                }
            }
        }
    }
}

fn brace_delta(line: &str) -> usize {
    line.chars().filter(|ch| *ch == '{').count()
}

fn close_brace_count(line: &str) -> usize {
    line.chars().filter(|ch| *ch == '}').count()
}
