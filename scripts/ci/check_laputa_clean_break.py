#!/usr/bin/env python3
"""Fail when removed legacy-memory runtime dependencies re-enter product code."""

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[2]
SCAN_ROOTS = (
    ROOT / "Cargo.toml",
    ROOT / "Cargo.lock",
    ROOT / "justfile",
    ROOT / ".github",
    ROOT / "agent-diva-core",
    ROOT / "agent-diva-agent",
    ROOT / "agent-diva-manager",
    ROOT / "agent-diva-cli",
    ROOT / "agent-diva-gui",
    ROOT / "agent-diva-laputa",
    ROOT / "agent-diva-autodream",
)
# The migration crate is scanned only for removed-module residue, not for the
# mentle pattern: its typed_memory.rs intentionally rejects `.mentle` import
# paths, so the generic runtime scan would false-positive there.
MIGRATION_SCAN_ROOTS = (ROOT / "agent-diva-migration",)
TEXT_SUFFIXES = {".rs", ".toml", ".lock", ".yml", ".yaml", ".json", ".ts", ".vue", ".md"}
REMOVED_RUNTIME = re.compile(r"\b(?:mentle|memtle)\b", re.IGNORECASE)
NATIVE_TOOLCHAIN = re.compile(r"\b(?:clang-cl|llvm)\b", re.IGNORECASE)
# Dead migration modules removed in GMH-50 (S2a). Match module usage only
# (`mod config_migration;` or `config_migration::`), never the unrelated
# `memory_migration_id` / `memory_migration_conflict` error-code identifiers
# in agent-diva-laputa/src/error.rs.
REMOVED_MIGRATION_MODULE = re.compile(
    r"\bmod\s+(?:config_migration|memory_migration|session_migration)\b"
    r"|\b(?:config_migration|memory_migration|session_migration)\s*::"
)


def files_to_scan(roots=SCAN_ROOTS):
    for root in roots:
        if root.is_file():
            yield root
            continue
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            if any(part in {"target", "node_modules", "dist"} for part in path.parts):
                continue
            if path.suffix.lower() in TEXT_SUFFIXES:
                yield path


def scan(roots, patterns):
    found = []
    for path in files_to_scan(roots):
        relative = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path
        text = path.read_text(encoding="utf-8", errors="replace")
        for line_number, line in enumerate(text.splitlines(), start=1):
            if any(pattern.search(line) for pattern in patterns):
                found.append(f"{relative}:{line_number}")
    return found


def self_test() -> int:
    """Positive/negative check that the residue detectors actually fire."""
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp) / "residue"
        (root / "src").mkdir(parents=True)
        (root / "src" / "lib.rs").write_text(
            "mod config_migration;\n"
            "config_migration::apply();\n"
            "session_migration::rollback();\n",
            encoding="utf-8",
        )
        (root / "src" / "unrelated.rs").write_text(
            'fn classify() { let x = "memory_migration_id"; }\n',
            encoding="utf-8",
        )
        patterns = (REMOVED_MIGRATION_MODULE,)
        found = [f for f in scan([root], patterns) if "lib.rs" in f]
        all_found = scan([root], patterns)
        if not found:
            print("self-test FAIL: config_migration:: module usage not detected")
            return 1
        if any("unrelated.rs" in f for f in all_found):
            print("self-test FAIL: memory_migration_id false positive")
            return 1
        print("self-test passed: residue detector fires and avoids error-code identifiers")
        return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    violations = scan(SCAN_ROOTS, (REMOVED_RUNTIME, NATIVE_TOOLCHAIN, REMOVED_MIGRATION_MODULE))
    violations += scan(MIGRATION_SCAN_ROOTS, (REMOVED_MIGRATION_MODULE,))
    if violations:
        print("Embedded Laputa / migration module clean-break violations:")
        print("\n".join(violations))
        return 1
    print("Embedded Laputa / migration module clean-break verified")
    return 0


if __name__ == "__main__":
    sys.exit(main())