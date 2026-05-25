#!/usr/bin/env python3
"""Verify Agent-Diva consumes Mentle from the frozen crates.io package."""

from pathlib import Path
import re
import sys


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    cargo_toml = (root / "Cargo.toml").read_text(encoding="utf-8")
    expected = 'memtle = { version = "0.1.2", default-features = false }'
    if expected not in cargo_toml:
        sys.exit(
            "workspace Cargo.toml must pin memtle to the published crates.io "
            "package at 0.1.2 with default-features = false"
        )

    if "[patch.crates-io]" in cargo_toml and "memtle" in cargo_toml.split(
        "[patch.crates-io]", 1
    )[1]:
        sys.exit("workspace Cargo.toml must not override memtle through [patch.crates-io]")

    for manifest in root.rglob("Cargo.toml"):
        text = manifest.read_text(encoding="utf-8")
        if re.search(r"(?ms)\[.*dependencies\.memtle.*?\]\s+.*\bpath\s*=", text):
            sys.exit(f"{manifest} must not define memtle as a path dependency")
        if re.search(r"(?ms)\[.*dependencies\.memtle.*?\]\s+.*\bgit\s*=", text):
            sys.exit(f"{manifest} must not define memtle as a git dependency")
        if re.search(r"(?m)^\s*memtle\s*=\s*\{[^}]*\bpath\s*=", text):
            sys.exit(f"{manifest} must not use an inline path override for memtle")
        if re.search(r"(?m)^\s*memtle\s*=\s*\{[^}]*\bgit\s*=", text):
            sys.exit(f"{manifest} must not use an inline git override for memtle")

    lock = (root / "Cargo.lock").read_text(encoding="utf-8")
    package_pattern = re.compile(
        r'\[\[package\]\]\nname = "memtle"\nversion = "0\.1\.2"\nsource = "registry\+https://github\.com/rust-lang/crates\.io-index"',
        re.MULTILINE,
    )
    if not package_pattern.search(lock):
        sys.exit("Cargo.lock must resolve memtle 0.1.2 from the crates.io registry")

    print("Mentle package policy verified: crates.io memtle 0.1.2")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
