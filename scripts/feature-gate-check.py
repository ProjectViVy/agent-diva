#!/usr/bin/env python3
"""Cross-platform feature gate compilation checks."""

from __future__ import annotations

import platform
import subprocess
import sys


def cargo_check(args: list[str], label: str) -> bool:
    print(f"  CHECK {label} ... ", end="", flush=True)
    result = subprocess.run(["cargo", *args], check=False)
    if result.returncode == 0:
        print("PASS")
        return True

    print(f"FAIL (exit {result.returncode})")
    return False


def main() -> int:
    system = platform.system().lower()
    host_platform_feature = {
        "windows": "platform-windows",
        "darwin": "platform-macos",
        "linux": "platform-linux",
    }.get(system)

    if host_platform_feature is None:
        print(f"Unsupported host platform for feature gate check: {system}", file=sys.stderr)
        return 1

    tests: list[tuple[str, list[str]]] = [
        (
            "agent-diva-sandbox --no-default-features",
            ["check", "-p", "agent-diva-sandbox", "--no-default-features"],
        ),
        (
            "agent-diva-sandbox --no-default-features --features filesystem",
            ["check", "-p", "agent-diva-sandbox", "--no-default-features", "--features", "filesystem"],
        ),
        (
            f"agent-diva-sandbox --no-default-features --features {host_platform_feature}",
            [
                "check",
                "-p",
                "agent-diva-sandbox",
                "--no-default-features",
                "--features",
                host_platform_feature,
            ],
        ),
        (
            "agent-diva-sandbox --no-default-features --features platform",
            ["check", "-p", "agent-diva-sandbox", "--no-default-features", "--features", "platform"],
        ),
        (
            "agent-diva-sandbox --no-default-features --features manager,platform",
            [
                "check",
                "-p",
                "agent-diva-sandbox",
                "--no-default-features",
                "--features",
                "manager,platform",
            ],
        ),
        (
            "agent-diva-sandbox --no-default-features --features approval,manager,platform",
            [
                "check",
                "-p",
                "agent-diva-sandbox",
                "--no-default-features",
                "--features",
                "approval,manager,platform",
            ],
        ),
        (
            "agent-diva-sandbox --no-default-features --features guardian,approval,manager,platform",
            [
                "check",
                "-p",
                "agent-diva-sandbox",
                "--no-default-features",
                "--features",
                "guardian,approval,manager,platform",
            ],
        ),
        (
            "agent-diva-sandbox --no-default-features --features orchestrator,platform",
            [
                "check",
                "-p",
                "agent-diva-sandbox",
                "--no-default-features",
                "--features",
                "orchestrator,platform",
            ],
        ),
    ]

    print("=== Feature Gate Check ===")
    print(f"Host platform: {system}")
    print(f"Testing {len(tests)} feature combinations\n")

    failed: list[str] = []
    for label, args in tests:
        if not cargo_check(args, label):
            failed.append(label)

    print("\n=== Summary ===")
    print(f"  Passed: {len(tests) - len(failed)}")
    print(f"  Failed: {len(failed)}")
    if failed:
        print("\n  Failed checks:")
        for label in failed:
            print(f"    - {label}")
        return 1

    print("\n  All feature gates passed!")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
