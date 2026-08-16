#!/usr/bin/env python3
"""Fail when D4 §3.1 legacy cognitive surfaces re-enter production paths."""

from __future__ import annotations

from pathlib import Path
import re
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]

SCAN_ROOTS = (
    ROOT / "Cargo.toml",
    ROOT / "justfile",
    ROOT / ".github",
    ROOT / "agent-diva-core",
    ROOT / "agent-diva-agent",
    ROOT / "agent-diva-manager",
    ROOT / "agent-diva-cli",
    ROOT / "agent-diva-gui",
    ROOT / "agent-diva-laputa",
    ROOT / "agent-diva-autodream",
    ROOT / "agent-diva-tools",
)

SKIP_DIR_NAMES = {
    "target",
    "node_modules",
    "dist",
    ".git",
}
TEXT_SUFFIXES = {".rs", ".toml", ".yml", ".yaml", ".json", ".ts", ".vue", ".js", ".md", ".just"}

# D4 §3.1 families. Keep patterns specific enough not to hit BML LongTerm,
# HTTP envelope JSON, or the working_memory tool gate.
FAMILIES = (
    (
        "legacy-save",
        re.compile(
            r"create_user_edit_proposal|laputa_propose_section_write|/api/laputa/section"
        ),
    ),
    (
        "legacy-http",
        re.compile(
            r"/api/laputa/proposals|/persona-workspace|/api/bml/memories"
        ),
    ),
    (
        "memory-governance",
        re.compile(
            r"MemoryGovernanceCoordinator|Capability::MemoryApply|ApprovalDomain::Memory"
            r"|Capability::MemoryPropose"
            r"|domain\s*[:=]\s*['\"]memory['\"]"
            r"|domain=memory"
        ),
    ),
    (
        "mixed-proposal",
        re.compile(
            r"ProposalType::(?:MemoryPatch|IdentityPatch|RelationshipUpdate|"
            r"CommitmentSet|LearningNote|SopCreate|Deprecation)\b"
        ),
    ),
    (
        "legacy-adapter",
        re.compile(
            r"\bDegradedMemoryProvider\b|\bCutoverMemoryProvider\b|"
            r"\bLegacyCrudMemoryProvider\b|MemoryManager::new\("
        ),
    ),
    (
        "world-governance",
        re.compile(
            r"\bWorldGovernance\b|WorldStore::project|world-ledger\.jsonl|world-proposals\.json"
        ),
    ),
    (
        "legacy-gui",
        re.compile(
            r"PersonaLifecyclePanel|GovernanceActionBar|ProposalInbox|"
            r"\bSectionEditor\b|formatJson|openEvolutionProposal"
        ),
    ),
    (
        "seed-retire",
        re.compile(
            r"\binitialize_sections\b|FIRST_RUN_ONBOARDING_BLOCK|"
            r"\bpersona_retire\b|persona-retire"
        ),
    ),
    (
        "prompt-approval",
        re.compile(r"not effective until approved"),
    ),
    (
        "memory-md",
        re.compile(r"LaputaSectionName::MemoryMd"),
    ),
    (
        "legacy-storage",
        re.compile(r"governance\.sqlite3"),
    ),
)

FALSIFICATION = re.compile(
    r"assert!\s*\(\s*!|!\s*prompt\.contains\(|removedKeys|"
    r"StatusCode::NOT_FOUND|must stay uncreated|must not|"
    r"does not exist|no longer|already gone|"
    r"keep_legacy_laputa_governance_routes_removed|"
    r"!registry\.has\(|classification:\s*'REMOVED'|"
    r"['\"]laputa\.(?:formatJson|sections)",
    re.IGNORECASE,
)

URI_LIST_LITERAL = re.compile(
    r"""^\s*["']/(?:api/laputa|api/bml|persona-workspace)[^"']*["'],?\s*$"""
)

# Path prefixes allowed to mention retired import vocabulary or schema seams.
PATH_ALLOW = (
    # Offline import adapters keep LaputaSectionName including MemoryMd.
    "agent-diva-core/src/evolution/types.rs",
    "agent-diva-laputa/src/memory_records.rs",
    # BML schema-preserving governed seam (D4 §3.2).
    "agent-diva-laputa/src/typed_store.rs",
    "agent-diva-laputa/src/bml/mod.rs",
    "agent-diva-laputa/tests/bml_boundary_guard.rs",
)


def posix(path: Path) -> str:
    return path.as_posix()


def files_to_scan(roots=SCAN_ROOTS):
    for root in roots:
        if not root.exists():
            continue
        if root.is_file():
            yield root
            continue
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            if any(part in SKIP_DIR_NAMES for part in path.parts):
                continue
            suffix = path.suffix.lower()
            if suffix in TEXT_SUFFIXES or path.name == "justfile":
                yield path


def is_path_allowed(relative: str) -> bool:
    return any(relative.replace("\\", "/") == allowed or relative.replace("\\", "/").startswith(allowed + "/") for allowed in PATH_ALLOW)


def is_falsification_line(line: str) -> bool:
    return bool(FALSIFICATION.search(line) or URI_LIST_LITERAL.match(line))


def scan(roots=SCAN_ROOTS):
    found = []
    for path in files_to_scan(roots):
        relative = posix(path.relative_to(ROOT) if path.is_relative_to(ROOT) else path)
        if is_path_allowed(relative):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for line_number, line in enumerate(text.splitlines(), start=1):
            if is_falsification_line(line):
                continue
            for family, pattern in FAMILIES:
                if pattern.search(line):
                    found.append(f"{relative}:{line_number} [{family}] {line.strip()}")
                    break
    return found


def self_test() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp) / "prod"
        src = root / "src"
        src.mkdir(parents=True)
        (src / "bad.rs").write_text(
            "let section = LaputaSectionName::MemoryMd;\n"
            "const EDITOR: &str = \"formatJson\";\n"
            "let domain = \"domain=memory\";\n"
            "fn x() { MemoryGovernanceCoordinator::new(); }\n",
            encoding="utf-8",
        )
        (src / "ok.rs").write_text(
            'assert!(!registry.has("laputa_propose_section_write"));\n'
            "const removedKeys = ['laputa.formatJson'];\n"
            '            "/api/laputa/proposals",\n'
            '        assert!(!prompt.contains("not effective until approved"));\n',
            encoding="utf-8",
        )
        hits = scan([root])
        bad = [hit for hit in hits if "bad.rs" in hit]
        ok = [hit for hit in hits if "ok.rs" in hit]
        needed = ("memory-md", "legacy-gui", "memory-governance")
        missing = [name for name in needed if not any(f"[{name}]" in hit for hit in bad)]
        # domain=memory is not a family by itself; MemoryGovernanceCoordinator covers governance.
        if missing:
            print("self-test FAIL: missing families:", ", ".join(missing))
            print("\n".join(bad) or "(no bad.rs hits)")
            return 1
        if ok:
            print("self-test FAIL: falsification lines were treated as residue:")
            print("\n".join(ok))
            return 1
        print("self-test passed: residue detector fires and skips falsification lines")
        return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    violations = scan()
    if violations:
        print("Cognitive clean-break violations (D4 §3.1 production path):")
        print("\n".join(violations))
        return 1
    print("Cognitive clean-break verified: D4 §3.1 families absent from production paths")
    return 0


if __name__ == "__main__":
    sys.exit(main())
