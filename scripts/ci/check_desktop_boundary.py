#!/usr/bin/env python3
"""Enforce the DIVA Go-host dependency and browser/native boundaries."""

from __future__ import annotations

import argparse
import re
import tempfile
from pathlib import Path


VIVY_IMPORT = re.compile(r"['\"](?:agent-vivy|github\.com/ProjectViVy/agent-vivy)/([^'\"]+)['\"]")
WAILS_IMPORT = re.compile(r"(?:from\s*|import\s*)['\"]@wailsio/runtime['\"]")
NATIVE_CALL = re.compile(r"\b(?:Window|Browser|Events|Application|Dialog|Clipboard)\s*\.\s*\w+\s*\(")
NATIVE_GLOBAL = re.compile(r"\bwindow\s*\.\s*(?:go|runtime|_wails)\s*(?:\.|\[)")
PROVIDER_FETCH = re.compile(
    r"fetch\s*\(\s*(?:[^)]*(?:api\.openai\.com|api\.deepseek\.com|api\.siliconflow\.cn|"
    r"api\.minimax|api\.anthropic\.com|api\.moonshot\.cn|api\.x\.ai)|"
    r"(?:resolved\.)?endpoint\b)",
    re.IGNORECASE,
)
PROVIDER_HOST = re.compile(
    r"api\.openai\.com|api\.deepseek\.com|api\.siliconflow\.cn|api\.minimax|"
    r"api\.anthropic\.com|api\.moonshot\.cn|api\.x\.ai",
    re.IGNORECASE,
)
COMMENTS = re.compile(r"/\*[\s\S]*?\*/|//[^\n]*")
AUTHORIZED_NATIVE_FILES = {
    "src/platform/desktop-host.ts",
    "src/utils/openExternal.ts",
    "src/api/vivy/transport.ts",
    "src/features/diva-pet/components/DesktopPetOverlay.vue",
}
DORMANT_PREFIX = "src/features/diva-pet/"
FORBIDDEN_ARTIFACT_NAME = re.compile(
    r"^(?:agent-vivy(?:\.(?:exe|bin))?|vivy(?:-shared|-host|-runtime)?"
    r"(?:\.(?:exe|dll|so|dylib|a|lib|bin))?|libvivy(?:[-_].*)?\.(?:dll|so|dylib|a|lib)|"
    r"laputa(?:\.(?:exe|bin))?)$",
    re.IGNORECASE,
)


def _frontend_files(root: Path):
    source = root / "agent-diva-gui" / "src"
    if not source.exists():
        return
    for path in sorted(source.rglob("*")):
        if not path.is_file() or path.suffix not in {".ts", ".tsx", ".js", ".vue"}:
            continue
        rel = path.relative_to(root / "agent-diva-gui").as_posix()
        if "/generated/" in f"/{rel}/" or path.name.endswith((".test.ts", ".spec.ts", ".d.ts")):
            continue
        yield path, rel


def check_desktop_boundary(root: Path, artifact: Path | None = None) -> list[str]:
    """Return all boundary violations in a DIVA source tree and optional artifact."""
    root = Path(root)
    violations: list[str] = []
    for base in (root / "internal", root / "cmd"):
        if not base.exists():
            continue
        for source in sorted(base.rglob("*.go")):
            text = source.read_text(encoding="utf-8", errors="replace")
            rel = source.relative_to(root).as_posix()
            for match in VIVY_IMPORT.finditer(text):
                package = match.group(1)
                if package != "sdk/host/v1":
                    violations.append(f"{rel}: VIVY internal import {package}")

    for source, rel in _frontend_files(root) or ():
        text = source.read_text(encoding="utf-8", errors="replace")
        code = COMMENTS.sub("", text)
        dormant = rel.startswith(DORMANT_PREFIX)
        if WAILS_IMPORT.search(code) and rel not in AUTHORIZED_NATIVE_FILES:
            violations.append(f"{rel}: direct native Wails import outside authorized seam")
        if NATIVE_CALL.search(code) and rel not in AUTHORIZED_NATIVE_FILES:
            violations.append(f"{rel}: direct native call outside authorized seam")
        if NATIVE_GLOBAL.search(code) and rel not in AUTHORIZED_NATIVE_FILES:
            violations.append(f"{rel}: direct native call outside authorized seam")
        if not dormant and PROVIDER_FETCH.search(text):
            violations.append(f"{rel}: browser provider fetch bypasses the Go host")
        elif not dormant and PROVIDER_HOST.search(text) and "fetch(" in text.replace(" ", ""):
            violations.append(f"{rel}: browser provider endpoint used with fetch")

    if artifact is not None and Path(artifact).exists():
        for path in sorted(Path(artifact).rglob("*")):
            if path.is_file() and FORBIDDEN_ARTIFACT_NAME.fullmatch(path.name):
                violations.append(f"{path.relative_to(artifact).as_posix()}: sidecar/second runtime")
    return violations


def _selftest() -> int:
    with tempfile.TemporaryDirectory(prefix="diva-boundary-selftest-") as tmp:
        root = Path(tmp)
        (root / "internal/desktop").mkdir(parents=True)
        (root / "agent-diva-gui/src/platform").mkdir(parents=True)
        (root / "agent-diva-gui/src/api").mkdir()
        (root / "internal/desktop/app.go").write_text(
            'package desktop\nimport "github.com/ProjectViVy/agent-vivy/sdk/host/v1"\n', encoding="utf-8"
        )
        (root / "agent-diva-gui/src/platform/desktop-host.ts").write_text(
            "import { Events } from '@wailsio/runtime'\n", encoding="utf-8"
        )
        if check_desktop_boundary(root):
            print("selftest: clean authorized seams failed")
            return 1
        cases = [
            ("vivy-import", root / "internal/desktop/escape.go", 'package desktop\nimport "agent-vivy/internal/runtime"\n', None, "VIVY internal import"),
            ("native-call", root / "agent-diva-gui/src/api/native.ts", "import { Window } from '@wailsio/runtime'\nWindow.Minimize()\n", None, "direct native"),
            ("provider-fetch", root / "agent-diva-gui/src/api/provider.ts", "fetch('https://api.openai.com/v1/chat/completions')\n", None, "provider fetch"),
        ]
        fired = 0
        for _name, path, body, artifact, expected in cases:
            path.write_text(body, encoding="utf-8")
            if any(expected in item for item in check_desktop_boundary(root, artifact)):
                fired += 1
            path.unlink()
        artifact = root / "artifact/lib"
        artifact.mkdir(parents=True)
        (artifact / "libvivy-shared.so").write_bytes(b"sidecar")
        if any("sidecar/second runtime" in item for item in check_desktop_boundary(root, artifact.parent)):
            fired += 1
        if fired != 4:
            print(f"selftest: {fired}/4 negative fixtures rejected")
            return 1
        print("selftest: 4/4 negative fixtures rejected; authorized SDK/native seams pass")
        return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--artifact", type=Path)
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest:
        return _selftest()
    violations = check_desktop_boundary(args.root, args.artifact)
    if violations:
        print("DESKTOP BOUNDARY VIOLATIONS:")
        for item in violations:
            print(f"  {item}")
        return 1
    print("desktop boundary clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
