#!/usr/bin/env python3
"""DN-P/DN-8 backend boundary gate.

Verifies the new product's backend boundary at three layers:

1. Rust dependency graph (Cargo.lock): only the thin-shell + vivy-bridge
   closure — no legacy agent-diva-* business crates.
2. Loader/resource manifest (tauri.conf.json): resources are exactly the
   vivy-runtime staging dir; no externalBin sidecars; the only invoke
   handler list is the vivy_call passthrough plus the DN-6A native
   speech surface (DN-6A state + DN-6B request lane; checked in src/lib.rs).
3. Packaged contents (.deb / --bundle dir): one shell binary, the sealed
   vivy-runtime set, no legacy executables, no extra domain databases.

Not a whole-tree ban on Rust/Tauri names — checks the actual dependency
graph, manifest fields and package file list.

Usage:
  python scripts/ci/check_vivy_backend_boundary.py [--src-tauri DIR]
          [--bundle PATH_OR_DIR] [--strict-package]

Exit 0 = clean; 1 = violations; findings print to stdout.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # py<3.11
    try:
        import tomli as tomllib  # type: ignore
    except ModuleNotFoundError:
        tomllib = None


def _toml(text: str) -> dict:
    if tomllib is not None:
        return tomllib.loads(text)
    # minimal fallback: enough for Cargo.lock/Cargo.toml key + [[package]] names
    data: dict = {}
    section: list[str] = []
    array_mode: str | None = None
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith('#'):
            continue
        if line.startswith('[[') and line.endswith(']]'):
            key = line[2:-2].strip()
            array_mode = key
            data.setdefault(key, []).append({})
            continue
        if line.startswith('[') and line.endswith(']'):
            section = [p.strip() for p in line[1:-1].split('.')]
            array_mode = None
            node = data
            for part in section:
                node = node.setdefault(part, {})
            continue
        if '=' in line:
            k, _, v = line.partition('=')
            k, v = k.strip(), v.strip().strip('"')
            if array_mode:
                data[array_mode][-1][k] = v
            else:
                node = data
                for part in section:
                    node = node[part]
                if v == '{}':
                    node[k] = {}
                elif v.startswith('['):
                    node[k] = [x.strip().strip('"') for x in v[1:-1].split(',') if x.strip()]
                else:
                    node[k] = v
    return data

REPO = Path(__file__).resolve().parents[2]

# Legacy business crates that must never re-enter the shipped closure.
# agent-diva-shell itself is the thin shell and is allowed.
FORBIDDEN_CRATES = {
    "agent-diva-core", "agent-diva-agent", "agent-diva-providers",
    "agent-diva-channels", "agent-diva-tools", "agent-diva-files",
    "agent-diva-tooling", "agent-diva-neuron", "agent-diva-manager",
    "agent-diva-autodream", "agent-diva-laputa", "agent-diva-sandbox",
    "agent-diva-nano", "agent-diva-cli", "agent-diva-service",
    "agent-diva-migration",
}

REQUIRED_RUNTIME_FILES = {"vivy-shared.so", "vivy_abi.h", "generation.json"}

DB_PATTERNS = re.compile(r"\.(db|sqlite|sqlite3|db3)$", re.IGNORECASE)
EXECUTABLE_BAD_NAMES = re.compile(
    r"(agent-diva(?!-shell)|gateway|manager|diva-cli|diva-service)", re.IGNORECASE
)

findings: list[str] = []


def fail(msg: str) -> None:
    findings.append(msg)


def ok(msg: str) -> None:
    print(f"[ok] {msg}")


def check_dep_graph(src_tauri: Path) -> None:
    lock = src_tauri / "Cargo.lock"
    if not lock.exists():
        fail(f"Cargo.lock missing: {lock}")
        return
    data = _toml(lock.read_text())
    names = {p["name"] for p in data.get("package", [])}
    bad = sorted(names & FORBIDDEN_CRATES)
    if bad:
        fail(f"legacy business crates in dependency graph: {bad}")
    else:
        ok(f"dependency graph clean ({len(names)} packages, no legacy crates)")

    manifest = src_tauri / "Cargo.toml"
    root = _toml(manifest.read_text())
    deps = set(root.get("dependencies", {})) | set(root.get("build-dependencies", {}))
    bad = sorted(deps & FORBIDDEN_CRATES)
    if bad:
        fail(f"legacy crates declared in Cargo.toml: {bad}")
    else:
        ok(f"Cargo.toml deps = {sorted(deps)}")


def check_manifest(src_tauri: Path) -> None:
    conf = json.loads((src_tauri / "tauri.conf.json").read_text())
    bundle = conf.get("bundle", {})
    resources = bundle.get("resources", [])
    if isinstance(resources, str):
        resources = [resources]
    extra = [r for r in resources if "vivy-runtime" not in str(r)]
    if extra:
        fail(f"unexpected bundle resources: {extra}")
    elif resources:
        ok(f"bundle resources = {resources}")
    else:
        fail("bundle.resources empty — vivy-runtime not packaged")

    ext = bundle.get("externalBin") or []
    if ext:
        fail(f"externalBin sidecars present (legacy executables?): {ext}")
    else:
        ok("no externalBin sidecars")

    lib = (src_tauri / "src" / "lib.rs").read_text()
    m = re.search(r"generate_handler!\[([^\]]+)\]", lib, re.S)
    handlers = [h.strip() for h in m.group(1).split(",") if h.strip()] if m else []
    # Native allowlist: vivy_call passthrough + DN-6A speech state +
    # DN-6B speech request-lane commands.
    # DN-6B adds transcribe/synthesize/cancel/context_set; any other
    # handler is a boundary violation.
    allowed = [
        "vivy_call",
        "speech::commands::speech_config_get",
        "speech::commands::speech_config_update",
        "speech::commands::speech_credential_set",
        "speech::commands::speech_credential_delete",
        "speech::commands::voice_asset_import",
        "speech::commands::voice_asset_list",
        "speech::commands::voice_asset_read",
        "speech::commands::voice_asset_delete",
        "speech::commands::speech_context_set",
        "speech::commands::speech_transcribe",
        "speech::commands::speech_synthesize",
        "speech::commands::speech_cancel",
    ]
    if sorted(handlers) != sorted(allowed):
        fail(f"invoke handlers {handlers} != allowed native surface {allowed}")
    else:
        ok(f"invoke handlers = native allowlist ({len(allowed)} commands)")


def _list_deb(deb: Path) -> list[str]:
    out = subprocess.run(
        ["dpkg-deb", "-c", str(deb)], capture_output=True, text=True, check=True
    ).stdout
    return [ln.split(None, 5)[5].lstrip("./") for ln in out.splitlines() if ln.strip()]


def _check_fileset(files: list[str], label: str) -> None:
    bins = [f for f in files if f.startswith("usr/bin/")]
    bad_bins = [f for f in bins if EXECUTABLE_BAD_NAMES.search(f)]
    if bad_bins:
        fail(f"{label}: legacy executables in bin/: {bad_bins}")
    else:
        ok(f"{label}: bin contents {bins}")

    runtime_files = {f.split("/", 1)[1] for f in files if f.startswith("usr/lib/DIVA/vivy-runtime/") or f.startswith("lib/DIVA/vivy-runtime/")}
    missing = REQUIRED_RUNTIME_FILES - {Path(f).name for f in runtime_files}
    if missing:
        fail(f"{label}: vivy-runtime missing {sorted(missing)}")
    else:
        ok(f"{label}: vivy-runtime carries {sorted(REQUIRED_RUNTIME_FILES)}")

    extra_so = [
        f for f in files
        if f.endswith(".so") and "vivy-runtime" not in f and not f.startswith("usr/lib/x86_64-linux-gnu/")
    ]
    if extra_so:
        fail(f"{label}: unexpected shared objects outside vivy-runtime: {extra_so}")

    dbs = [f for f in files if DB_PATTERNS.search(f)]
    if dbs:
        fail(f"{label}: domain databases bundled: {dbs}")
    else:
        ok(f"{label}: no bundled databases")


def check_package(bundle: Path) -> None:
    if bundle.is_dir():
        files = [str(p.relative_to(bundle)) for p in bundle.rglob("*") if p.is_file()]
        _check_fileset(["usr/" + f if not f.startswith("usr/") else f for f in files], str(bundle))
        return
    if bundle.suffix == ".deb":
        _check_fileset(_list_deb(bundle), bundle.name)
        return
    if bundle.suffix in (".rpm",):
        out = subprocess.run(
            ["rpm", "-qlp", str(bundle)], capture_output=True, text=True, check=True
        ).stdout
        _check_fileset([ln.strip().lstrip("/") for ln in out.splitlines()], bundle.name)
        return
    # generic dir/executable list via tar
    if bundle.suffixes[-2:] == [".tar", ".gz"] or bundle.suffix == ".zip":
        tmp = tempfile.mkdtemp()
        if bundle.suffix == ".zip":
            subprocess.run(["unzip", "-q", str(bundle), "-d", tmp], check=True)
        else:
            subprocess.run(["tar", "-xzf", str(bundle), "-C", tmp], check=True)
        check_package(Path(tmp))
        return
    fail(f"unsupported bundle format: {bundle}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--src-tauri", default=str(REPO / "agent-diva-gui" / "src-tauri"))
    ap.add_argument("--bundle", default=None,
                    help="packaged artifact (.deb/.rpm/dir). Auto-detects deb under target/release/bundle/deb.")
    ap.add_argument("--strict-package", action="store_true",
                    help="fail when no package is found to inspect")
    a = ap.parse_args()

    src_tauri = Path(a.src_tauri)
    check_dep_graph(src_tauri)
    check_manifest(src_tauri)

    bundle = Path(a.bundle) if a.bundle else None
    if bundle is None:
        debs = sorted((src_tauri / "target" / "release" / "bundle" / "deb").glob("*.deb"))
        bundle = debs[-1] if debs else None
    if bundle and bundle.exists():
        check_package(bundle)
    elif a.strict_package:
        fail("no packaged artifact found for --strict-package")
    else:
        ok("package-contents check skipped (no bundle)")

    if findings:
        print("\nBOUNDARY VIOLATIONS:")
        for f in findings:
            print(f"  - {f}")
        return 1
    print("\nboundary gate clean")
    return 0


if __name__ == "__main__":
    sys.exit(main())
