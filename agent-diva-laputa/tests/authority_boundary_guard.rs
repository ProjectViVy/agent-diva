use std::{fs, path::Path};

#[derive(Clone, Copy)]
pub struct ForbiddenPattern {
    needle: &'static str,
}

impl ForbiddenPattern {
    pub const fn new(needle: &'static str) -> Self {
        ForbiddenPattern { needle }
    }
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
    "agent-diva-gui/src-tauri/src",
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

const READ_CALL_MARKERS: &[&str] = &[
    "fs::read_to_string(",
    "std::fs::read_to_string(",
    "fs::read(",
    "std::fs::read(",
    "tokio::fs::read_to_string(",
    "tokio::fs::read(",
    "file::open(",
];

pub fn assert_authority_boundaries() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    let write_violations = scan_forbidden_access(
        repo_root,
        STATIC_SCAN_ROOTS,
        WRITE_CALL_MARKERS,
        FORBIDDEN_PATTERNS,
        &[],
    );
    assert!(
        write_violations.is_empty(),
        "unauthorized durable authority writes found:\n{}",
        write_violations.join("\n")
    );

    let read_violations = scan_forbidden_access(
        repo_root,
        &["agent-diva-agent/src", "agent-diva-autodream/src"],
        READ_CALL_MARKERS,
        FORBIDDEN_PATTERNS,
        &[],
    );
    assert!(
        read_violations.is_empty(),
        "unauthorized runtime authority reads found:\n{}",
        read_violations.join("\n")
    );
}

pub fn scan_forbidden_access(
    repo_root: &Path,
    roots: &[&str],
    operation_markers: &[&str],
    forbidden_patterns: &[ForbiddenPattern],
    snippet_allowlist: &[(&str, &str)],
) -> Vec<String> {
    let mut violations = Vec::new();

    for root in roots {
        let root_path = repo_root.join(root);
        collect_rs_files(&root_path, &mut |path| {
            let relative = relative_to_repo(repo_root, path);
            let raw = fs::read_to_string(path).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", path.display());
            });
            let stripped = strip_test_modules(&raw);
            let lines = stripped.lines().collect::<Vec<_>>();

            for index in 0..lines.len() {
                let upper = (index + 4).min(lines.len());
                let snippet = lines[index..upper].join("\n");
                let lowered = snippet.to_ascii_lowercase();
                if !operation_markers
                    .iter()
                    .any(|token| lowered.contains(token))
                {
                    continue;
                }
                if !forbidden_patterns
                    .iter()
                    .any(|pattern| lowered.contains(pattern.needle))
                {
                    continue;
                }
                if snippet_allowlist
                    .iter()
                    .any(|(allowed_path, allowed_snippet)| {
                        *allowed_path == relative
                            && lowered.contains(&allowed_snippet.to_ascii_lowercase())
                    })
                {
                    continue;
                }

                violations.push(format!(
                    "{relative}:{}: forbidden access pattern detected near:\n{}",
                    index + 1,
                    snippet.trim()
                ));
            }
        });
    }

    violations
}

fn collect_rs_files(root: &Path, visit: &mut dyn FnMut(&Path)) {
    if !root.exists() {
        return;
    }

    let entries = fs::read_dir(root).unwrap_or_else(|error| {
        panic!("failed to read directory {}: {error}", root.display());
    });

    for entry in entries {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, visit);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            visit(&path);
        }
    }
}

fn relative_to_repo(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn strip_test_modules(content: &str) -> String {
    let mut result = Vec::new();
    let mut pending_test_attr = false;
    let mut skip_depth = 0usize;

    for line in content.lines() {
        let trimmed = line.trim();

        if skip_depth > 0 {
            skip_depth = next_depth(skip_depth, line);
            if skip_depth == 0 {
                pending_test_attr = false;
            }
            continue;
        }

        if trimmed == "#[cfg(test)]" {
            pending_test_attr = true;
            continue;
        }

        if pending_test_attr && trimmed.starts_with("mod tests") {
            skip_depth = brace_delta(line);
            if skip_depth == 0 {
                pending_test_attr = false;
            }
            continue;
        }

        pending_test_attr = false;
        result.push(line);
    }

    result.join("\n")
}

fn next_depth(depth: usize, line: &str) -> usize {
    let opens = line.chars().filter(|ch| *ch == '{').count();
    let closes = line.chars().filter(|ch| *ch == '}').count();
    depth + opens - closes
}

fn brace_delta(line: &str) -> usize {
    let opens = line.chars().filter(|ch| *ch == '{').count();
    let closes = line.chars().filter(|ch| *ch == '}').count();
    opens.saturating_sub(closes)
}
