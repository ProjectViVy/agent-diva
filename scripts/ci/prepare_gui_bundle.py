#!/usr/bin/env python3
"""Stage GUI bundle resources for Tauri installers.

This script copies the release CLI binary and any optional service binary into
`agent-diva-gui/src-tauri/resources/` so the Tauri bundler can embed them into
the desktop installers. It intentionally tolerates a missing service binary so
the GUI installer can evolve ahead of the Windows service crate.
"""

from __future__ import annotations

import argparse
import json
import platform
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path


TARGET_OS_MAP = {
    "windows": "windows",
    "darwin": "macos",
    "linux": "linux",
}


def detect_target_os(value: str | None) -> str:
    if value:
        normalized = value.strip().lower()
        aliases = {"mac": "macos", "macos": "macos", "darwin": "macos"}
        return aliases.get(normalized, normalized)

    detected = platform.system().lower()
    return TARGET_OS_MAP.get(detected, detected)


def binary_name(name: str, target_os: str) -> str:
    if target_os == "windows" and not name.endswith(".exe"):
        return f"{name}.exe"
    return name


def copy_file(src: Path, dst: Path) -> None:
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst)


def copy_tree(src: Path, dst: Path) -> None:
    if not src.exists():
        return
    shutil.copytree(src, dst, dirs_exist_ok=True)


def build_manifest(
    *,
    target_os: str,
    cli_source: Path,
    service_source: Path | None,
    destination_root: Path,
) -> dict:
    return {
        "prepared_at_utc": datetime.now(timezone.utc).isoformat(),
        "target_os": target_os,
        "resource_root": str(destination_root),
        "binaries": {
            "cli": {
                "source": str(cli_source),
                "staged": str(destination_root / "bin" / target_os / cli_source.name),
                "required": True,
            },
            "service": {
                "source": str(service_source) if service_source else None,
                "staged": (
                    str(destination_root / "bin" / target_os / service_source.name)
                    if service_source
                    else None
                ),
                "required": False,
            },
        },
        "service_templates": {
            "linux_systemd": str(destination_root / "systemd")
            if target_os == "linux"
            else None,
            "macos_launchd": str(destination_root / "launchd")
            if target_os == "macos"
            else None,
        },
    }


def assert_required_cli_staged(cli_path: Path) -> None:
    if not cli_path.exists():
        raise FileNotFoundError(
            f"required staged CLI runtime missing after copy: {cli_path}"
        )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Stage CLI/service binaries into Tauri bundle resources."
    )
    parser.add_argument(
        "--gui-root",
        default="agent-diva-gui",
        help="Path to the agent-diva-gui directory.",
    )
    parser.add_argument(
        "--workspace-root",
        default=None,
        help="Optional workspace root. Defaults to the parent of --gui-root.",
    )
    parser.add_argument(
        "--target-os",
        default=None,
        choices=["windows", "linux", "macos", "darwin", "mac"],
        help="Override detected target OS for staging paths.",
    )
    parser.add_argument(
        "--cli-binary",
        default=None,
        help="Optional explicit path to the built CLI binary.",
    )
    parser.add_argument(
        "--service-binary",
        default=None,
        help="Optional explicit path to the built service binary.",
    )
    args = parser.parse_args()

    gui_root = Path(args.gui_root).resolve()
    if not gui_root.exists():
        print(f"[prepare_gui_bundle] GUI root not found: {gui_root}", file=sys.stderr)
        return 1

    workspace_root = (
        Path(args.workspace_root).resolve()
        if args.workspace_root
        else gui_root.parent.resolve()
    )
    target_os = detect_target_os(args.target_os)

    cli_source = (
        Path(args.cli_binary).resolve()
        if args.cli_binary
        else workspace_root / "target" / "release" / binary_name("agent-diva", target_os)
    )
    if not cli_source.exists():
        print(
            f"[prepare_gui_bundle] Required CLI binary not found: {cli_source}",
            file=sys.stderr,
        )
        return 1

    service_source = (
        Path(args.service_binary).resolve()
        if args.service_binary
        else workspace_root
        / "target"
        / "release"
        / binary_name("agent-diva-service", target_os)
    )
    if not service_source.exists():
        service_source = None

    resources_root = gui_root / "src-tauri" / "resources"
    staged_bin_root = resources_root / "bin"
    manifest_root = resources_root / "manifests"
    shutil.rmtree(staged_bin_root, ignore_errors=True)
    shutil.rmtree(manifest_root, ignore_errors=True)

    staged_cli_path = staged_bin_root / target_os / cli_source.name
    copy_file(cli_source, staged_cli_path)
    assert_required_cli_staged(staged_cli_path)

    if service_source:
        staged_service_path = staged_bin_root / target_os / service_source.name
        copy_file(service_source, staged_service_path)

    if target_os == "linux":
        systemd_src = workspace_root / "contrib" / "systemd"
        systemd_dst = resources_root / "systemd"
        shutil.rmtree(systemd_dst, ignore_errors=True)
        copy_tree(systemd_src, systemd_dst)

    if target_os == "macos":
        launchd_src = workspace_root / "contrib" / "launchd"
        launchd_dst = resources_root / "launchd"
        shutil.rmtree(launchd_dst, ignore_errors=True)
        copy_tree(launchd_src, launchd_dst)

    manifest_root.mkdir(parents=True, exist_ok=True)
    manifest = build_manifest(
        target_os=target_os,
        cli_source=cli_source,
        service_source=service_source,
        destination_root=resources_root,
    )
    manifest_path = manifest_root / "gui-bundle-manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
    )

    readme_path = staged_bin_root / target_os / "README.txt"
    readme_path.write_text(
        "\n".join(
            [
                "These binaries are staged for Tauri installer packaging.",
                "agent-diva(.exe) is required because the GUI manages the local gateway runtime.",
                "agent-diva-service(.exe) is an optional advanced component for Windows service mode.",
            ]
        )
        + "\n",
        encoding="utf-8",
    )

    print(f"[prepare_gui_bundle] staged CLI binary: {staged_cli_path}")
    if service_source:
        print(f"[prepare_gui_bundle] staged service binary: {staged_bin_root / target_os / service_source.name}")
    else:
        print("[prepare_gui_bundle] service binary missing; continuing without it")
    print(f"[prepare_gui_bundle] wrote manifest: {manifest_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
