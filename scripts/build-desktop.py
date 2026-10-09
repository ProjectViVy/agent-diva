#!/usr/bin/env python3
"""DIVA desktop build wrapper (DN-W3 / W3-4).

Drives the sealed go-host pack target through the VIVY SDK. Two modes:

    build  — frozen pnpm install/build once, then `sdk pack --target go-host`
             and `sdk inspect-artifact`. Release sources are required by
             default; --development snapshots dirty inputs and is labeled
             non-release in the emitted lock.
    test   — stages the same dependency/modfile closure into a task-local
             directory (pack with an empty asset tree supplies the resolved
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
import copy
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import tempfile
from pathlib import Path
from typing import Any

LOCK_SCHEMA = "diva.go-host-inputs/v1"
RECIPE = "recipes/diva.vivy.yml"
HOST_PACKAGE = "./cmd/diva"
HOST_ASSETS = "agent-diva-gui/dist"
INOFY_MODULE = "github.com/ProjectViVy/inofy"
PLATFORMS = ("linux-amd64", "windows-amd64")
LOCK_COMMON_TOOL_KEYS = ("go", "wails", "node", "pnpm")


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
    `relpath\x00mode\x00` + payload + `\x00`). File modes are canonicalized
    to git index values (regulars: index perm & 0o777, dirs/gitlinks: 0o755,
    links: 0o777) because lstat perms are platform-dependent (0o666/0o777 on
    Windows vs index-derived 0o644/0o755 on Linux) and would make the digest
    unreproducible cross-platform."""
    modes: dict[str, int] = {}
    for entry in git(root, "ls-files", "-s", "-z").split("\x00"):
        if not entry:
            continue
        meta, _, path = entry.partition("\t")
        fields = meta.split()
        if not fields or not path:
            continue
        modes[path] = int(fields[0], 8)
    entries = sorted(modes)
    h = hashlib.sha256()
    for rel in entries:
        full = root / rel
        st = os.lstat(full)
        index_mode = modes.get(rel, 0)
        is_link = stat.S_ISLNK(st.st_mode) or index_mode & 0o170000 == 0o120000
        is_dir = stat.S_ISDIR(st.st_mode) or index_mode & 0o170000 == 0o160000
        if is_link:
            mode = 0o777
        elif is_dir:
            mode = 0o755
        else:
            mode = index_mode & 0o777
        h.update(rel.encode())
        h.update(b"\x00")
        h.update(("%o" % mode).encode())
        h.update(b"\x00")
        if is_link:
            try:
                target: bytes = os.readlink(full).encode()
            except OSError:
                # A checkout without symlink support materializes the link
                # as a text file containing the target — the git blob
                # payload, which hashes identically.
                target = full.read_bytes()
            h.update(b"link\x00")
            h.update(target)
            h.update(b"\x00")
        elif is_dir:
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
    """Resolve VIVY's inofy module pin to its full origin commit."""
    for line in (vivy_root / "go.mod").read_text().splitlines():
        parts = line.split()
        if len(parts) >= 2 and parts[0] == INOFY_MODULE:
            version = parts[1]
            raw = run(["go", "mod", "download", "-json", f"{INOFY_MODULE}@{version}"], cwd=vivy_root)
            try:
                metadata = json.loads(raw)
            except json.JSONDecodeError as exc:
                raise SystemExit(f"cannot decode go mod download metadata for {INOFY_MODULE}@{version}: {exc}")
            commit = str(metadata.get("Origin", {}).get("Hash", ""))
            suffix = version.rsplit("-", 1)[-1] if "-" in version else ""
            if not re.fullmatch(r"[0-9a-f]{40}", commit) or not suffix or not commit.startswith(suffix):
                raise SystemExit(f"{INOFY_MODULE}@{version} has no matching full Origin.Hash")
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


def canonical_lock_bytes(lock: dict[str, Any]) -> bytes:
    """Canonical JSON for lock identity and deterministic derivation."""
    return (json.dumps(lock, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode("utf-8")


def load_source_lock(path: Path) -> dict[str, Any]:
    try:
        lock = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot read source lock {path}: {exc}")
    if lock.get("schema") != LOCK_SCHEMA:
        raise SystemExit(f"source lock schema must be {LOCK_SCHEMA}")
    if not isinstance(lock.get("tools", {}).get("platforms"), dict):
        raise SystemExit("source lock tools.platforms is required")
    if set(lock["tools"]["platforms"]) != set(PLATFORMS):
        raise SystemExit(f"source lock tools.platforms must be exactly {PLATFORMS}")
    return lock


def derive_target_lock(source_lock: dict[str, Any], target: str) -> dict[str, Any]:
    """Derive one native target lock from the sole canonical source lock."""
    if target not in PLATFORMS:
        raise ValueError(f"unsupported desktop target {target!r}")
    platforms = source_lock.get("tools", {}).get("platforms", {})
    target_fields = platforms.get(target)
    if not isinstance(target_fields, dict):
        raise ValueError(f"source lock has no tools.platforms.{target}")
    for field in ("goos", "goarch", "cgo", "buildTags"):
        if field not in target_fields:
            raise ValueError(f"source lock tools.platforms.{target} is missing {field}")
    if not isinstance(target_fields["goos"], str) or not isinstance(target_fields["goarch"], str):
        raise ValueError(f"source lock tools.platforms.{target} GOOS/GOARCH must be strings")
    if not isinstance(target_fields["cgo"], bool):
        raise ValueError(f"source lock tools.platforms.{target} cgo must be boolean")
    if not isinstance(target_fields["buildTags"], list) or not all(isinstance(tag, str) for tag in target_fields["buildTags"]):
        raise ValueError(f"source lock tools.platforms.{target} buildTags must be a string list")
    derived = copy.deepcopy(source_lock)
    tools = derived.setdefault("tools", {})
    for field in ("goos", "goarch", "cgo"):
        tools[field] = target_fields[field]
    tools["platform"] = target
    tools["sourceLockSHA256"] = hashlib.sha256(canonical_lock_bytes(source_lock)).hexdigest()
    derived["buildTags"] = list(target_fields["buildTags"])
    return derived


def verify_derived_lock(source_lock: dict[str, Any], derived_lock: dict[str, Any], target: str) -> list[str]:
    try:
        expected = derive_target_lock(source_lock, target)
    except ValueError as exc:
        return [str(exc)]
    if canonical_lock_bytes(derived_lock) != canonical_lock_bytes(expected):
        return [f"derived lock drift for {target}: generated bytes differ from source-lock derivation"]
    return []


def validate_native_platform(requested: str, actual_goos: str, actual_goarch: str) -> list[str]:
    parts = requested.split("-", 1)
    if len(parts) != 2:
        return [f"invalid requested platform {requested!r}"]
    expected_goos, expected_goarch = parts
    if (actual_goos, actual_goarch) != (expected_goos, expected_goarch):
        return [
            f"requested platform {requested} does not match native Go target "
            f"{actual_goos}/{actual_goarch}"
        ]
    return []


def validate_locked_platform(
    source_lock: dict[str, Any], requested: str, actual_goos: str, actual_goarch: str, actual_cgo: str
) -> list[str]:
    errors = validate_native_platform(requested, actual_goos, actual_goarch)
    target = source_lock.get("tools", {}).get("platforms", {}).get(requested)
    if not isinstance(target, dict):
        return errors + [f"source lock has no target declaration for {requested}"]
    if not isinstance(target.get("goos"), str) or not isinstance(target.get("goarch"), str):
        errors.append(f"source-lock target {requested} GOOS/GOARCH must be strings")
    if (target.get("goos"), target.get("goarch")) != (actual_goos, actual_goarch):
        errors.append(
            f"source-lock target {requested} declares {target.get('goos')}/{target.get('goarch')}, "
            f"native toolchain is {actual_goos}/{actual_goarch}"
        )
    if not isinstance(target.get("cgo"), bool):
        errors.append(f"source-lock target {requested} cgo must be boolean")
    elif target["cgo"] != (actual_cgo == "1"):
        errors.append(f"native CGO_ENABLED={actual_cgo} does not match source lock {target.get('cgo')}")
    return errors


def build_lock(args: argparse.Namespace, release: bool) -> dict[str, Any]:
    """Build a source-only lock using the tracked tool/platform declarations."""
    vivy_root = Path(args.vivy_dir).resolve()
    laputa_root = Path(args.laputa_dir).resolve()
    host_root = Path(args.host_dir).resolve()
    template = load_source_lock(host_root / "build" / "vivy-sources.lock.json")
    recipe = vivy_root / RECIPE
    lock = copy.deepcopy(template)
    lock.update({
        "schema": LOCK_SCHEMA,
        "release": release,
        "host": {
            "modulePath": "github.com/ProjectViVy/agent-diva",
            "repository": remote_url(host_root) or "https://github.com/ProjectViVy/agent-diva",
            # DIVA's own commit/tree intentionally resolve at pack time.
        },
    })
    lock["sources"] = {
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
    }
    lock["recipe"] = {"path": RECIPE, "sha256": sha256_file(recipe)}
    lock["locks"] = {
        "go.mod": sha256_file(host_root / "go.mod"),
        "go.sum": sha256_file(host_root / "go.sum"),
        "pnpm-lock.yaml": sha256_file(host_root / "agent-diva-gui" / "pnpm-lock.yaml"),
    }
    return lock


def _go_environment(cwd: Path) -> tuple[str, str, str]:
    values = run(["go", "env", "GOOS", "GOARCH", "CGO_ENABLED"], cwd=cwd).splitlines()
    if len(values) != 3:
        raise SystemExit(f"unexpected go env output: {values!r}")
    return values[0], values[1], values[2]


def declared_tool_errors(source_lock: dict[str, Any], host_root: Path) -> list[str]:
    """Ensure manifest, Go module, Wails runtime and source-lock pins agree."""
    errors: list[str] = []
    tools = source_lock.get("tools", {})
    package = json.loads((host_root / "agent-diva-gui" / "package.json").read_text(encoding="utf-8"))
    package_manager = str(package.get("packageManager", "")).split("+", 1)[0]
    if package_manager != f"pnpm@{tools.get('pnpm')}":
        errors.append("agent-diva-gui/package.json packageManager does not match source lock")
    runtime_version = package.get("dependencies", {}).get("@wailsio/runtime")
    if runtime_version != str(tools.get("wails", "")).removeprefix("v"):
        errors.append("@wailsio/runtime does not match source-lock Wails version")
    go_mod = (host_root / "go.mod").read_text(encoding="utf-8")
    wails_module = re.search(r"(?m)^\s*github\.com/wailsapp/wails/v3\s+(v[^\s]+)", go_mod)
    if not wails_module or wails_module.group(1) != tools.get("wails"):
        errors.append("Go Wails module does not match source-lock Wails version")
    return errors


def _actual_tool_errors(source_lock: dict[str, Any], host_root: Path) -> list[str]:
    errors = declared_tool_errors(source_lock, host_root)
    tools = source_lock.get("tools", {})
    actual_go = go_version()
    if actual_go != tools.get("go"):
        errors.append(f"Go version {actual_go} does not match lock {tools.get('go')}")
    actual_node = run(["node", "--version"], cwd=host_root).lstrip("v")
    if actual_node.split(".", 1)[0] != str(tools.get("node", "")).split(".", 1)[0]:
        errors.append(f"Node version {actual_node} does not match lock {tools.get('node')}")
    actual_pnpm = run(["pnpm", "--version"], cwd=host_root)
    if actual_pnpm != str(tools.get("pnpm", "")):
        errors.append(f"pnpm version {actual_pnpm} does not match lock {tools.get('pnpm')}")
    try:
        wails_output = run(["wails3", "version"], cwd=host_root)
        expected_wails = str(tools.get("wails", ""))
        if expected_wails not in wails_output:
            errors.append(f"wails3 version output does not contain pinned {expected_wails}")
    except SystemExit as exc:
        errors.append(str(exc))
    return errors


def check_source_lock(source_lock: dict[str, Any], args: argparse.Namespace) -> list[str]:
    """Compare repository inputs and installed tools with the canonical lock."""
    host_root = Path(args.host_dir).resolve()
    current = build_lock(args, release=bool(source_lock.get("release", False)))
    errors: list[str] = []
    for field in ("host", "sources", "recipe", "locks"):
        if current.get(field) != source_lock.get(field):
            errors.append(f"current {field} inputs differ from canonical source lock")
    for key in LOCK_COMMON_TOOL_KEYS:
        if current.get("tools", {}).get(key) != source_lock.get("tools", {}).get(key):
            errors.append(f"source lock common tool {key} changed during derivation")
    goos, goarch, cgo = _go_environment(host_root)
    errors.extend(validate_locked_platform(source_lock, getattr(args, "platform", ""), goos, goarch, cgo))
    errors.extend(_actual_tool_errors(source_lock, host_root))
    return errors


def mode_check_lock(args: argparse.Namespace) -> None:
    if not getattr(args, "platform", None):
        raise SystemExit("--mode check-lock requires --platform")
    host_root = Path(args.host_dir).resolve()
    source = load_source_lock(host_root / "build" / "vivy-sources.lock.json")
    errors = check_source_lock(source, args)
    if args.derived_lock:
        try:
            derived = json.loads(Path(args.derived_lock).read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            errors.append(f"cannot read generated target lock: {exc}")
        else:
            errors.extend(verify_derived_lock(source, derived, args.platform))
    if errors:
        raise SystemExit("lock check failed:\n- " + "\n- ".join(errors))
    print(f"source and derived locks verified for {args.platform}")


def write_artifact_locks(output: Path, source_lock: dict[str, Any], derived_lock: dict[str, Any]) -> None:
    files = {
        "source-inputs.lock.json": canonical_lock_bytes(source_lock),
        "input-lock.json": canonical_lock_bytes(derived_lock),
    }
    for name, body in files.items():
        (output / name).write_bytes(body)
    checksum_path = output / "checksums.sha256"
    entries = checksum_path.read_text(encoding="utf-8").splitlines() if checksum_path.exists() else []
    entries = [line for line in entries if line.split("  ", 1)[-1] not in files]
    entries.extend(f"{hashlib.sha256(body).hexdigest()}  {name}" for name, body in files.items())
    checksum_path.write_text("\n".join(sorted(entries)) + "\n", encoding="utf-8")


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

    if not getattr(args, "development", False):
        for name, root in (("host", host_root), ("vivy", Path(args.vivy_dir).resolve()), ("laputa", Path(args.laputa_dir).resolve())):
            dirty = git(root, "status", "--porcelain")
            if dirty:
                raise SystemExit(f"release build requires clean sources; {name} is dirty:\n{dirty}")

    source_lock = build_lock(args, release=False) if getattr(args, "development", False) else load_source_lock(host_root / "build" / "vivy-sources.lock.json")
    goos, goarch, cgo = _go_environment(host_root)
    target_name = getattr(args, "platform", None) or f"{goos}-{goarch}"
    native_errors = validate_locked_platform(source_lock, target_name, goos, goarch, cgo)
    if native_errors:
        raise SystemExit("build target check failed:\n- " + "\n- ".join(native_errors))
    if not getattr(args, "development", False):
        check_args = argparse.Namespace(**vars(args))
        check_args.platform = target_name
        errors = check_source_lock(source_lock, check_args)
        if errors:
            raise SystemExit("lock check failed:\n- " + "\n- ".join(errors))
    try:
        target_lock = derive_target_lock(source_lock, target_name)
    except ValueError as exc:
        raise SystemExit(str(exc))
    drift = verify_derived_lock(source_lock, target_lock, target_name)
    if drift:
        raise SystemExit("generated target lock failed deterministic check:\n- " + "\n- ".join(drift))

    # One frontend build, from the frozen lockfile. Pack binds these bytes.
    run(["pnpm", "--dir", str(gui), "install", "--frozen-lockfile"], cwd=host_root)
    run(["pnpm", "--dir", str(gui), "build"], cwd=host_root)
    # Vite empties dist/ and drops the tracked .gitkeep the embed needs on a
    # fresh checkout — restore it after every build.
    (gui / "dist" / ".gitkeep").touch()

    with tempfile.TemporaryDirectory(prefix="diva-target-lock-") as lock_dir:
        lock_path = Path(lock_dir) / "input-lock.json"
        lock_path.write_bytes(canonical_lock_bytes(target_lock))
        sdk_pack(args, lock_path, HOST_ASSETS, output)
    write_artifact_locks(output, source_lock, target_lock)
    run(["go", "run", "./sdk", "inspect-artifact", str(output)], cwd=Path(args.vivy_dir).resolve())
    print(f"sealed go-host artifact: {output}")
    print(f"build report: {output / 'build-report.json'}")


def mode_test(args: argparse.Namespace) -> None:
    host_root = Path(args.host_dir).resolve()
    source_lock = build_lock(args, release=False)
    goos, goarch, cgo = _go_environment(host_root)
    target_name = getattr(args, "platform", None) or f"{goos}-{goarch}"
    native_errors = validate_locked_platform(source_lock, target_name, goos, goarch, cgo)
    if native_errors:
        raise SystemExit("test target check failed:\n- " + "\n- ".join(native_errors))
    try:
        target_lock = derive_target_lock(source_lock, target_name)
    except ValueError as exc:
        raise SystemExit(str(exc))
    drift = verify_derived_lock(source_lock, target_lock, target_name)
    if drift:
        raise SystemExit("generated target lock failed deterministic check:\n- " + "\n- ".join(drift))
    stage = Path(tempfile.mkdtemp(prefix="diva-gohost-stage-"))
    staged_host = stage / "host"
    staged_vivy = stage / "vivy"
    staged_laputa = stage / "deps" / "laputa"
    stage_tracked(host_root, staged_host)
    stage_tracked(Path(args.vivy_dir).resolve(), staged_vivy)
    stage_tracked(Path(args.laputa_dir).resolve(), staged_laputa)

    # Pack supplies the resolved consumer modfile: an empty asset tree under
    # --host-dir keeps the frontend out of test mode entirely.
    empty_in_host = host_root / ".test-empty-assets"
    empty_in_host.mkdir(exist_ok=True)
    (empty_in_host / "index.html").write_text(
        "<!doctype html><html><head><meta charset=\"utf-8\"></head><body></body></html>\n",
        encoding="utf-8",
    )
    artifact = stage / "artifact"
    lock_path = stage / "vivy-sources.lock.json"
    lock_path.write_bytes(canonical_lock_bytes(target_lock))
    sdk_pack(args, lock_path, ".test-empty-assets", artifact)
    shutil.copy2(artifact / "consumer.mod", staged_host / "consumer.mod")
    shutil.copy2(artifact / "consumer.sum", staged_host / "consumer.sum")
    tags = " ".join(["vivy_headless", *target_lock.get("buildTags", [])])
    packages = getattr(args, "test_packages", None) or ["./..."]
    test_args = ["-run", args.test_run] if getattr(args, "test_run", "") else []
    go_common = ["-modfile", "consumer.mod", "-mod=readonly", "-tags", tags]
    run(["go", "build", *go_common, *packages], cwd=staged_host)
    run(["go", "test", "-race", *test_args, *go_common, *packages], cwd=staged_host)
    print(f"go race tests passed under the sealed consumer modfile (stage: {stage})")


def mode_repin(args: argparse.Namespace) -> None:
    host_root = Path(args.host_dir).resolve()
    lock = build_lock(args, release=True)
    target = host_root / "build" / "vivy-sources.lock.json"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(lock, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"repinned {target}")


def main() -> None:
    repo = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description="DIVA sealed desktop build wrapper (W3-4)")
    parser.add_argument("--mode", choices=["build", "test", "repin", "check-lock"], default="build")
    parser.add_argument("--development", action="store_true", help="snapshot dirty inputs; output is non-release")
    parser.add_argument("--platform", choices=PLATFORMS, help="native target; must match the running Go toolchain")
    parser.add_argument("--derived-lock", help="generated target lock to verify in check-lock mode")
    parser.add_argument("--test-run", default="", help="optional Go test name regular expression")
    parser.add_argument("--test-packages", nargs="+", help="optional Go packages for mode=test")
    parser.add_argument("--host-dir", default=str(repo))
    parser.add_argument("--vivy-dir", default=str(repo.parent / "agent-vivy"))
    parser.add_argument("--laputa-dir", default=str(repo.parent / "laputa"))
    parser.add_argument("--output", default=str(repo.parent / "artifacts" / "diva-go-host"))
    args = parser.parse_args()
    {"build": mode_build, "test": mode_test, "repin": mode_repin, "check-lock": mode_check_lock}[args.mode](args)


if __name__ == "__main__":
    main()
