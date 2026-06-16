use std::{fs, path::Path};

const WRITE_ROOTS: &[&str] = &[
    "agent-diva-agent/src",
    "agent-diva-autodream/src",
    "agent-diva-gui/src-tauri/src",
];

const READ_ROOTS: &[&str] = &["agent-diva-agent/src", "agent-diva-autodream/src"];

const WRITE_OPS: &[&str] = &[
    "std::fs::write",
    "fs::write",
    "tokio::fs::write",
    "openoptions::new",
    "file::create",
];

const READ_OPS: &[&str] = &[
    "std::fs::read_to_string",
    "std::fs::read(",
    "fs::read_to_string",
    "fs::read(",
    "tokio::fs::read_to_string",
    "tokio::fs::read(",
    "file::open",
];

const FORBIDDEN_WRITE_TOKENS: &[&str] = &[
    ".laputa",
    "memory.md",
    "history.md",
    "identity.md",
    "soul.md",
    "user.md",
    "profile.md",
    "task.md",
    "bootstrap.md",
];

const FORBIDDEN_READ_TOKENS: &[&str] = &[
    "memory.md",
    "history.md",
    "identity.md",
    "soul.md",
    "user.md",
    "profile.md",
    "task.md",
    ".laputa/sections",
    ".laputa\\sections",
];

const WRITE_ALLOWLIST: &[&str] = &[
    "agent-diva-autodream/src/atomic.rs",
    "agent-diva-autodream/src/layout.rs",
    "agent-diva-autodream/src/outputs.rs",
    "agent-diva-autodream/src/service.rs",
    "agent-diva-autodream/src/worker.rs",
];

const READ_ALLOWLIST: &[(&str, &str)] = &[(
    "agent-diva-agent/src/context.rs",
    "read_soul_file(\"bootstrap.md\")",
)];

#[test]
fn direct_write_guard_limits_authority_writes_to_allowlisted_boundaries() {
    let violations = scan_forbidden_access(
        WRITE_ROOTS,
        WRITE_OPS,
        FORBIDDEN_WRITE_TOKENS,
        WRITE_ALLOWLIST,
        &[],
    );

    assert!(
        violations.is_empty(),
        "unauthorized durable authority writes found:\n{}",
        violations.join("\n")
    );
}

#[test]
fn direct_read_guard_limits_runtime_authority_reads_to_laputa_boundaries() {
    let violations = scan_forbidden_access(
        READ_ROOTS,
        READ_OPS,
        FORBIDDEN_READ_TOKENS,
        &[],
        READ_ALLOWLIST,
    );

    assert!(
        violations.is_empty(),
        "unauthorized runtime authority reads found:\n{}",
        violations.join("\n")
    );
}

fn scan_forbidden_access(
    roots: &[&str],
    operation_tokens: &[&str],
    forbidden_tokens: &[&str],
    file_allowlist: &[&str],
    snippet_allowlist: &[(&str, &str)],
) -> Vec<String> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let mut violations = Vec::new();

    for root in roots {
        let root_path = repo_root.join(root);
        collect_rs_files(&root_path, &mut |path| {
            let relative = relative_to_repo(repo_root, path);
            if file_allowlist.iter().any(|allowed| *allowed == relative) {
                return;
            }

            let raw = fs::read_to_string(path).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", path.display());
            });
            let stripped = strip_test_modules(&raw);
            let lines = stripped.lines().collect::<Vec<_>>();

            for index in 0..lines.len() {
                let upper = (index + 4).min(lines.len());
                let snippet = lines[index..upper].join("\n");
                let lowered = snippet.to_ascii_lowercase();
                if !operation_tokens.iter().any(|token| lowered.contains(token)) {
                    continue;
                }
                if !forbidden_tokens.iter().any(|token| lowered.contains(token)) {
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
