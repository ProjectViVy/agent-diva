#!/usr/bin/env python3
"""Fail if agent-diva-swarm dependency tree includes agent-diva-meta (ADR-A). Story 5.4 / CI gate."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    proc = subprocess.run(
        ["cargo", "tree", "-p", "agent-diva-swarm"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr or proc.stdout or "cargo tree failed\n")
        sys.exit(proc.returncode)
    out = proc.stdout or ""
    # 精确匹配 crate 名，避免误伤其它包名子串
    forbidden = "agent-diva-meta v"
    if forbidden in out or "agent-diva-meta " in out:
        sys.stderr.write(
            "ERROR: agent-diva-swarm must not depend on agent-diva-meta (ADR-A)\n"
        )
        sys.stderr.write(out)
        sys.exit(1)
    print("OK: agent-diva-swarm dependency tree has no agent-diva-meta")


if __name__ == "__main__":
    main()
