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
TEXT_SUFFIXES = {".rs", ".toml", ".lock", ".yml", ".yaml", ".json", ".ts", ".vue", ".md"}
REMOVED_RUNTIME = re.compile(r"\b(?:mentle|memtle)\b", re.IGNORECASE)
NATIVE_TOOLCHAIN = re.compile(r"\b(?:clang-cl|llvm)\b", re.IGNORECASE)


def files_to_scan():
    for root in SCAN_ROOTS:
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


def main() -> int:
    violations = []
    for path in files_to_scan():
        relative = path.relative_to(ROOT)
        text = path.read_text(encoding="utf-8", errors="replace")
        for line_number, line in enumerate(text.splitlines(), start=1):
            if REMOVED_RUNTIME.search(line) or NATIVE_TOOLCHAIN.search(line):
                violations.append(f"{relative}:{line_number}")
    if violations:
        print("Embedded Laputa clean-break violations:")
        print("\n".join(violations))
        return 1
    print("Embedded Laputa clean-break verified")
    return 0


if __name__ == "__main__":
    sys.exit(main())
