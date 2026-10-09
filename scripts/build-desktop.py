#!/usr/bin/env python3
"""DIVA desktop build wrapper (DN-W3 / W3-4).

Drives the sealed go-host pack target through the VIVY SDK. Two modes:

    build  — frozen pnpm install/build once, then `sdk pack --target go-host`
             and `sdk inspect-artifact`. Release sources are required by
             default; --development snapshots dirty inputs and is labeled
             non-release in the emitted lock.
    test   — stages the same dependency/modfile closure into a task-local
             directory (pack with a minimal test asset tree supplies the resolved
             consumer modfile), then runs Go build and `go test -race` with
             `-modfile consumer.mod -tags vivy_headless`. No frontend build
             is required and nothing claims a release artifact.

The tracked lock `build/vivy-sources.lock.json` never pins DIVA's own
commit; `--repin` regenerates it after dependency bumps. The pack tool
resolves the host commit/tree digest from the live checkout and records it
in build-report.json inside the artifact.

Layout the consumer modfile expects (matches the SDK staging layout):

    <stage>/host          staged DIVA checkout (host go.mod lives here)
    <stage>/vivy          staged agent-vivy checkout
    <stage>/deps/laputa   staged laputa checkout (garden/mentle/laputa modules)
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import platform
from pathlib import Path

LOCK_SCHEMA = "diva.go-host-inputs/v1"
RECIPE = "recipes/diva.vivy.yml"
HOST_PACKAGE = "./cmd/diva"
HOST_ASSETS = "agent-diva-gui/dist"
WAILS_VERSION = "v3.0.0-beta.27"
INOFY_MODULE = "github.com/ProjectViVy/inofy"


def run(args: list[str], cwd: Path, capture: bool = False) -> str:
    proc = subprocess.run(
        args, cwd=cwd, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
    )
    if proc.returncode != 0:
        raise SystemExit(f"{args[0]} {' '.join(args[1:])} failed in {cwd}:\n{proc.stdout}")
    return proc.stdout.strip()


def git(root: Path, *args: str) -> str:
    return run(["git", *args], cwd=root, capture=True)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_tree_hash(root: Path) -> str:
    """Byte-exact port of the SDK's hashSourceTree (git ls-files, per-entry
    `relpath\x00mode\x00` + payload + `\x00`)."""
    tracked = git(root, "ls-files", "-z").split("\x00")
    entries = sorted(p for p in tracked if p)
    h = hashlib.sha256()
    for rel in entries:
        full = root / rel
        st = os.lstat(full)
        mode = stat.S_IMODE(st.st_mode)
        # Windows Python lstat marks regular files with executable
        # extensions (.cmd/.bat/.exe) as +x while Go's os.Lstat reports
        # plain 0666/0444. Strip the synthetic exec bits so the digest
        # matches the SDK's hashSourceTree on Windows.
        if os.name == "nt" and stat.S_ISREG(st.st_mode):
            mode &= ~0o111
        h.update(rel.encode())
        h.update(b"\x00")
        h.update(("%o" % mode).encode())
        h.update(b"\x00")
        if stat.S_ISLNK(st.st_mode):
            h.update(b"link\x00")
            h.update(os.readlink(full).encode())
            h.update(b"\x00")
        elif stat.S_ISDIR(st.st_mode):
            try:
                sub = git(full, "rev-parse", "HEAD")
                h.update(b"gitlink\x00" + sub.encode() + b"\x00")
            except SystemExit:
                h.update(b"gitlink\x00absent\x00")
        else:
            body = full.read_bytes()
            h.update(str(len(body)).encode())
            h.update(b"\x00")
            h.update(body)
        h.update(b"\x00")
    return h.hexdigest()


_GIT_MANAGER_PROXY = "https://git-manager.devin.ai/proxy/"


def remote_url(root: Path) -> str:
    try:
        url = git(root, "remote", "get-url", "origin")
    except SystemExit:
        return ""
    # Record the canonical host URL, not the environment-local proxy.
    if url.startswith(_GIT_MANAGER_PROXY):
        url = "https://" + url[len(_GIT_MANAGER_PROXY):]
    return url


def inofy_pin(vivy_root: Path) -> dict:
    """Pull the pinned inofy module version/commit out of VIVY's go.mod."""
    for line in (vivy_root / "go.mod").read_text().splitlines():
        parts = line.split()
        if len(parts) >= 2 and parts[0] == INOFY_MODULE:
            version = parts[1]
            commit = version.rsplit("-", 1)[-1] if "-" in version else ""
            return {"module": INOFY_MODULE, "version": version, "commit": commit}
    raise SystemExit(f"{INOFY_MODULE} not pinned in {vivy_root / 'go.mod'}")


def go_version() -> str:
    out = run(["go", "version"], cwd=Path.cwd())
    # "go version go1.26.4 linux/amd64" -> "go1.26.4"
    return out.split()[2] if len(out.split()) >= 3 else out


def stage_tracked(src: Path, dst: Path) -> None:
    """Copy only `git ls-files` entries (no .git, no ignored outputs)."""
    dst.mkdir(parents=True, exist_ok=True)
    tracked = git(src, "ls-files", "-z").split("\x00")
    for rel in sorted(p for p in tracked if p):
        source = src / rel
        target = dst / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        st = os.lstat(source)
        if stat.S_ISLNK(st.st_mode):
            if target.exists() or target.is_symlink():
                target.unlink()
            os.symlink(os.readlink(source), target)
        elif stat.S_ISDIR(st.st_mode):
            target.mkdir(parents=True, exist_ok=True)
        else:
            shutil.copy2(source, target)


def build_lock(args: argparse.Namespace, release: bool) -> dict:
    vivy_root = Path(args.vivy_dir).resolve()
    laputa_root = Path(args.laputa_dir).resolve()
    host_root = Path(args.host_dir).resolve()
    recipe = vivy_root / RECIPE
    lock = {
        "schema": LOCK_SCHEMA,
        "release": release,
        "host": {
            "modulePath": "github.com/ProjectViVy/agent-diva",
            "repository": remote_url(host_root) or "https://github.com/ProjectViVy/agent-diva",
            # commit/treeSHA256 intentionally absent: the tracked lock must
            # not pin its own DIVA commit — pack resolves them at build time.
        },
        "sources": {
            "vivy": {
                "repository": remote_url(vivy_root) or "https://github.com/ProjectViVy/agent-vivy",
                "commit": git(vivy_root, "rev-parse", "HEAD"),
                "treeSHA256": source_tree_hash(vivy_root),
            },
            "laputa": {
                "repository": remote_url(laputa_root) or "https://github.com/ProjectViVy/laputa",
                "commit": git(laputa_root, "rev-parse", "HEAD"),
                "treeSHA256": source_tree_hash(laputa_root),
            },
            "inofy": inofy_pin(vivy_root),
        },
        "recipe": {"path": RECIPE, "sha256": sha256_file(recipe)},
        "locks": {
            "go.mod": sha256_file(host_root / "go.mod") if (host_root / "go.mod").exists() else "",
            "go.sum": sha256_file(host_root / "go.sum") if (host_root / "go.sum").exists() else "",
            "pnpm-lock.yaml": sha256_file(host_root / "agent-diva-gui" / "pnpm-lock.yaml"),
        },
        # Extra `go build` tags the host package needs beyond vivy_headless
        # (sealed into hostBuild.tools.buildTags; gtk3 is a no-op on
        # non-Linux platforms — its constraints are all *_linux files).
        "buildTags": ["gtk3"] if platform.system().lower() == "linux" else [],
        "tools": {
            "go": go_version(),
            "wails": WAILS_VERSION,
            "goos": {"linux": "linux", "darwin": "darwin"}.get(platform.system().lower(), "windows"),
            "goarch": {"x86_64": "amd64", "amd64": "amd64", "aarch64": "arm64", "arm64": "arm64"}.get(platform.machine().lower(), platform.machine().lower()),
            "cgo": True,
        },
        "keyring": {},
    }
    return lock


def sdk_pack(args: argparse.Namespace, lock_path: Path, assets: str, output: Path) -> None:
    vivy_root = Path(args.vivy_dir).resolve()
    run(
        [
            "go", "run", "./sdk", "pack", "--target", "go-host",
            "--recipe", RECIPE,
            "--host-dir", str(Path(args.host_dir).resolve()),
            "--host-package", HOST_PACKAGE,
            "--host-assets", assets,
            "--host-lock", str(lock_path),
            "--output", str(output),
        ],
        cwd=vivy_root,
    )


def mode_build(args: argparse.Namespace) -> None:
    host_root = Path(args.host_dir).resolve()
    gui = host_root / "agent-diva-gui"
    output = Path(args.output).resolve()
    if output.exists() and any(output.iterdir()):
        raise SystemExit(f"output directory already exists and is not empty: {output}")

    if not args.development:
        for name, root in (("host", host_root), ("vivy", Path(args.vivy_dir).resolve()), ("laputa", Path(args.laputa_dir).resolve())):
            dirty = git(root, "status", "--porcelain")
            if dirty:
                raise SystemExit(f"release build requires clean sources; {name} is dirty:\n{dirty}")

    # One frontend build, from the frozen lockfile. Pack binds these bytes.
    run(["pnpm", "--dir", str(gui), "install", "--frozen-lockfile"], cwd=host_root)
    run(["pnpm", "--dir", str(gui), "build"], cwd=host_root)
    # Vite empties dist/ and drops the tracked .gitkeep the embed needs on a
    # fresh checkout — restore it after every build.
    (gui / "dist" / ".gitkeep").touch()

    tracked = host_root / "build" / "vivy-sources.lock.json"
    if args.development:
        lock = build_lock(args, release=False)
        lock_path = Path(tempfile.mkdtemp(prefix="diva-dev-lock-")) / "vivy-sources.lock.json"
        lock_path.write_text(json.dumps(lock, indent=2) + "\n")
    else:
        lock_path = tracked
    sdk_pack(args, lock_path, HOST_ASSETS, output)
    run(["go", "run", "./sdk", "inspect-artifact", str(output)], cwd=Path(args.vivy_dir).resolve())
    print(f"sealed go-host artifact: {output}")
    print(f"build report: {output / 'build-report.json'}")


def mode_test(args: argparse.Namespace) -> None:
    host_root = Path(args.host_dir).resolve()
    stage = Path(tempfile.mkdtemp(prefix="diva-gohost-stage-"))
    staged_host = stage / "host"
    staged_vivy = stage / "vivy"
    staged_laputa = stage / "deps" / "laputa"
    stage_tracked(host_root, staged_host)
    stage_tracked(Path(args.vivy_dir).resolve(), staged_vivy)
    stage_tracked(Path(args.laputa_dir).resolve(), staged_laputa)

    artifact = stage / "artifact"
    lock = build_lock(args, release=False)
    lock_path = stage / "vivy-sources.lock.json"
    lock_path.write_text(json.dumps(lock, indent=2) + "\n")
    # The SDK requires an entry document even for backend-only tests. Keep
    # it task-local and remove it on success or failure; no frontend install
    # is needed and these bytes are never a release UI.
    with tempfile.TemporaryDirectory(prefix=".test-assets-", dir=host_root) as assets_dir:
        assets = Path(assets_dir)
        (assets / "index.html").write_text("<!doctype html><title>DIVA backend test</title>\n")
        sdk_pack(args, lock_path, assets.name, artifact)
    shutil.copy2(artifact / "consumer.mod", staged_host / "consumer.mod")
    shutil.copy2(artifact / "consumer.sum", staged_host / "consumer.sum")
    tags = " ".join(["vivy_headless", *lock.get("buildTags", [])])
    run(["go", "build", "-modfile", "consumer.mod", "-mod=readonly", "-tags", tags, "./..."], cwd=staged_host)
    run(["go", "test", "-race", "-modfile", "consumer.mod", "-mod=readonly", "-tags", tags, "./..."], cwd=staged_host)
    print(f"go race tests passed under the sealed consumer modfile (stage: {stage})")


def mode_repin(args: argparse.Namespace) -> None:
    host_root = Path(args.host_dir).resolve()
    lock = build_lock(args, release=True)
    target = host_root / "build" / "vivy-sources.lock.json"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(lock, indent=2) + "\n")
    print(f"repinned {target}")


def main() -> None:
    repo = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description="DIVA sealed desktop build wrapper (W3-4)")
    parser.add_argument("--mode", choices=["build", "test", "repin"], default="build")
    parser.add_argument("--development", action="store_true", help="snapshot dirty inputs; output is non-release")
    parser.add_argument("--host-dir", default=str(repo))
    parser.add_argument("--vivy-dir", default=str(repo.parent / "agent-vivy"))
    parser.add_argument("--laputa-dir", default=str(repo.parent / "laputa"))
    parser.add_argument("--output", default=str(repo.parent / "artifacts" / "diva-go-host"))
    args = parser.parse_args()
    {"build": mode_build, "test": mode_test, "repin": mode_repin}[args.mode](args)


if __name__ == "__main__":
    main()
