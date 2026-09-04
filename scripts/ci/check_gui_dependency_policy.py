#!/usr/bin/env python3
"""Static guards for the GUI package manager and dependency policy."""

from __future__ import annotations

import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
GUI_ROOT = ROOT / "agent-diva-gui"
GUI_PACKAGE = GUI_ROOT / "package.json"
AVATAR_PACKAGE = GUI_ROOT / "avatar-runtime-vrm" / "package.json"
GUI_LOCK = GUI_ROOT / "pnpm-lock.yaml"
CI_WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"

EXPECTED_SECURITY_VERSIONS = {
    "brace-expansion": "2.1.4",
    "browserslist": "4.28.7",
    "nanoid": "3.3.18",
    "postcss": "8.5.23",
}

ACTIVE_SURFACES = (
    ROOT / "justfile",
    ROOT / "scripts" / "package-windows-gui.ps1",
    ROOT / "scripts" / "build-macos-gui-bundle.sh",
    ROOT / "scripts" / "start-diva-olv-smoke.ps1",
    ROOT / "scripts" / "make-diva.ps1",
    CI_WORKFLOW,
    GUI_ROOT / "src-tauri" / "tauri.conf.json",
)

FORBIDDEN_ACTIVE_COMMAND = re.compile(
    r"(?i)(?:^|[\s\"'`=(])(?:npm|npx|yarn|yarnpkg)(?:[\s\"'`)]|$)"
)
PACKAGE_MANAGER = re.compile(r"^pnpm@(\d+\.\d+\.\d+)\+sha512\.[0-9a-f]+$")
LOCK_PACKAGE_KEY = re.compile(r"^  (?P<name>[^\s@]+)@(?P<version>[^:\s]+):", re.MULTILINE)


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def relative(path: Path) -> str:
    return str(path.relative_to(ROOT)).replace("\\", "/")


def package_manager_value(manifest: dict, path: Path, errors: list[str]) -> tuple[str, str] | None:
    value = manifest.get("packageManager")
    if not isinstance(value, str):
        errors.append(f"{relative(path)}: packageManager must be a pnpm value")
        return None
    match = PACKAGE_MANAGER.fullmatch(value)
    if not match:
        errors.append(f"{relative(path)}: packageManager must pin pnpm with sha512 integrity")
        return None
    return value, match.group(1)


def check_manifest_policy(errors: list[str]) -> tuple[str | None, str | None]:
    gui = load_json(GUI_PACKAGE)
    avatar = load_json(AVATAR_PACKAGE)
    gui_manager = package_manager_value(gui, GUI_PACKAGE, errors)
    avatar_manager = package_manager_value(avatar, AVATAR_PACKAGE, errors)

    if gui_manager and avatar_manager:
        if gui_manager[0] != avatar_manager[0]:
            errors.append(
                f"{relative(AVATAR_PACKAGE)}: packageManager differs from {relative(GUI_PACKAGE)}"
            )

    for path, manifest in ((GUI_PACKAGE, gui), (AVATAR_PACKAGE, avatar)):
        scripts = manifest.get("scripts", {})
        if not isinstance(scripts, dict):
            errors.append(f"{relative(path)}: scripts must be an object")
            continue
        for name, command in scripts.items():
            if isinstance(command, str) and FORBIDDEN_ACTIVE_COMMAND.search(command):
                errors.append(f"{relative(path)}: script {name!r} uses npm/yarn")

    if avatar.get("scripts", {}).get("build") != (
        "pnpm run build:lib && pnpm run build:demo"
    ):
        errors.append(f"{relative(AVATAR_PACKAGE)}: build script must use pnpm")

    for section in ("dependencies", "devDependencies", "optionalDependencies"):
        if "pnpm" in gui.get(section, {}):
            errors.append(f"{relative(GUI_PACKAGE)}: pnpm must not be bundled as a dependency")

    expected_overrides = EXPECTED_SECURITY_VERSIONS
    actual_overrides = gui.get("pnpm", {}).get("overrides", {})
    if actual_overrides != expected_overrides:
        errors.append(f"{relative(GUI_PACKAGE)}: dependency overrides do not match policy")

    version = gui_manager[1] if gui_manager else None
    return version, gui_manager[0] if gui_manager else None


def check_lock_policy(errors: list[str]) -> None:
    lock_text = GUI_LOCK.read_text(encoding="utf-8")
    expected_block = """overrides:
  brace-expansion: 2.1.4
  browserslist: 4.28.7
  nanoid: 3.3.18
  postcss: 8.5.23
"""
    if expected_block not in lock_text:
        errors.append(f"{relative(GUI_LOCK)}: expected override block is missing")

    package_keys = {
        match.group("name"): match.group("version")
        for match in LOCK_PACKAGE_KEY.finditer(lock_text)
        if match.group("name") in EXPECTED_SECURITY_VERSIONS
    }
    for name, expected in EXPECTED_SECURITY_VERSIONS.items():
        versions = [
            match.group("version")
            for match in LOCK_PACKAGE_KEY.finditer(lock_text)
            if match.group("name") == name
        ]
        if not versions or set(versions) != {expected}:
            errors.append(
                f"{relative(GUI_LOCK)}: {name} resolves to {sorted(set(versions))}, expected {expected}"
            )
        if package_keys.get(name) != expected:
            errors.append(f"{relative(GUI_LOCK)}: {name} package key is not pinned to {expected}")

    if "pnpm@" in lock_text:
        errors.append(f"{relative(GUI_LOCK)}: pnpm must not be installed as a GUI dependency")


def check_surface_policy(errors: list[str], version: str | None) -> None:
    for path in ACTIVE_SURFACES:
        text = path.read_text(encoding="utf-8")
        if FORBIDDEN_ACTIVE_COMMAND.search(text):
            errors.append(f"{relative(path)}: active npm/yarn command found")

    windows_script = (ROOT / "scripts" / "package-windows-gui.ps1").read_text(
        encoding="utf-8"
    )
    macos_script = (ROOT / "scripts" / "build-macos-gui-bundle.sh").read_text(
        encoding="utf-8"
    )
    for path, text in (
        (ROOT / "scripts" / "package-windows-gui.ps1", windows_script),
        (ROOT / "scripts" / "build-macos-gui-bundle.sh", macos_script),
    ):
        if "pnpm install --frozen-lockfile" not in text:
            errors.append(f"{relative(path)}: packaging install must be frozen")

    ci_text = CI_WORKFLOW.read_text(encoding="utf-8")
    if version and not re.search(rf"PNPM_VERSION:\s*['\"]{re.escape(version)}['\"]", ci_text):
        errors.append(f"{relative(CI_WORKFLOW)}: PNPM_VERSION does not match packageManager")

    audit_job_match = re.search(
        r"(?ms)^  gui-audit:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)",
        ci_text,
    )
    if not audit_job_match:
        errors.append(f"{relative(CI_WORKFLOW)}: gui-audit job is missing")
        return
    audit_job = audit_job_match.group("body")
    if not re.search(r"(?m)^\s+runs-on:\s+ubuntu-latest\s*$", audit_job):
        errors.append(f"{relative(CI_WORKFLOW)}: gui-audit must run once on Linux")
    if not re.search(r"(?m)^\s+timeout-minutes:\s+[1-9]\d*\s*$", audit_job):
        errors.append(f"{relative(CI_WORKFLOW)}: gui-audit timeout-minutes must be bounded")
    if "pnpm audit --audit-level=moderate" not in audit_job:
        errors.append(f"{relative(CI_WORKFLOW)}: moderate-level pnpm audit is missing")
    if not re.search(
        r"(?ms)- name: Audit all frontend dependencies\s+working-directory: agent-diva-gui\s+timeout-minutes: 3\s+run: pnpm audit --audit-level=moderate",
        audit_job,
    ):
        errors.append(f"{relative(CI_WORKFLOW)}: pnpm audit step must have a 3-minute timeout")
    if "--ignore-registry-errors" in audit_job or "continue-on-error: true" in audit_job:
        errors.append(f"{relative(CI_WORKFLOW)}: audit registry failures must fail closed")
    if re.search(r"pnpm audit[^\n]*(--prod|--dev|--no-optional)", audit_job):
        errors.append(f"{relative(CI_WORKFLOW)}: audit must include all dependency classes")
    if len(re.findall(r"pnpm audit --audit-level=moderate", ci_text)) != 1:
        errors.append(f"{relative(CI_WORKFLOW)}: audit must run once, not per OS")

    release_job_match = re.search(
        r"(?ms)^  release:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)",
        ci_text,
    )
    if not release_job_match or not re.search(r"needs:\s*\[.*gui-audit", release_job_match.group("body")):
        errors.append(f"{relative(CI_WORKFLOW)}: release must depend on gui-audit")


def check_lockfiles(errors: list[str]) -> None:
    ignored = {"node_modules", "dist", "demo-dist", "target", ".git"}
    alternate_names = {"package-lock.json", "yarn.lock", "npm-shrinkwrap.json"}
    for path in GUI_ROOT.rglob("*"):
        if not path.is_file() or any(part in ignored for part in path.parts):
            continue
        if path.name in alternate_names:
            errors.append(f"{relative(path)}: alternate lockfile must not exist")

    gitignore = (GUI_ROOT / ".gitignore").read_text(encoding="utf-8")
    if not re.search(r"(?m)^package-lock\.json\s*$", gitignore):
        errors.append(f"{relative(GUI_ROOT / '.gitignore')}: package-lock.json must stay ignored")


def main() -> int:
    errors: list[str] = []
    version, _ = check_manifest_policy(errors)
    check_lock_policy(errors)
    check_surface_policy(errors, version)
    check_lockfiles(errors)
    if errors:
        print("GUI dependency policy violations:")
        for error in errors:
            print(f"- {error}")
        return 1
    print("GUI dependency policy verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
