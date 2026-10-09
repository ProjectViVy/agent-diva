from __future__ import annotations

import importlib.util
import copy
import hashlib
import json
import os
import re
import stat
import subprocess
import tempfile
import unittest
from fnmatch import fnmatch
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "build-desktop.py"
SPEC = importlib.util.spec_from_file_location("build_desktop", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
BUILD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)
ROOT = SCRIPT.parents[1]


class DesktopBuildContractTests(unittest.TestCase):
    def test_test_mode_supplies_empty_embedded_index_before_sdk_pack(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            host = root / "host"
            vivy = root / "vivy"
            laputa = root / "laputa"
            for source in (host, vivy, laputa):
                source.mkdir()

            def stage_tracked(_source: Path, destination: Path) -> None:
                destination.mkdir(parents=True, exist_ok=True)

            def sdk_pack(args, _lock_path: Path, assets: str, output: Path) -> None:
                index = Path(args.host_dir) / assets / "index.html"
                self.assertTrue(index.is_file(), "test-mode SDK pack needs an embedded index.html")
                self.assertIn("<html", index.read_text(encoding="utf-8").lower())
                output.mkdir(parents=True, exist_ok=True)
                (output / "consumer.mod").write_text("module example.com/host\n", encoding="utf-8")
                (output / "consumer.sum").write_text("", encoding="utf-8")

            args = BUILD.argparse.Namespace(
                host_dir=str(host), vivy_dir=str(vivy), laputa_dir=str(laputa), output=str(root / "out"),
                platform="linux-amd64", test_run="TestRuntime", test_packages=["./internal/desktop"],
            )
            source_lock = {
                "schema": "diva.go-host-inputs/v1",
                "release": False,
                "tools": {"platforms": {
                    "linux-amd64": {"goos": "linux", "goarch": "amd64", "cgo": True, "buildTags": ["gtk3"]},
                    "windows-amd64": {"goos": "windows", "goarch": "amd64", "cgo": True, "buildTags": []},
                }},
            }
            calls: list[list[str]] = []
            def record_run(command: list[str], **_kwargs) -> str:
                calls.append(command)
                return ""

            with (
                patch.object(BUILD, "stage_tracked", side_effect=stage_tracked),
                patch.object(BUILD, "build_lock", return_value=source_lock),
                patch.object(BUILD, "_go_environment", return_value=("linux", "amd64", "1")),
                patch.object(BUILD, "sdk_pack", side_effect=sdk_pack),
                patch.object(BUILD, "run", side_effect=record_run),
            ):
                BUILD.mode_test(args)
            go_test = next(command for command in calls if command[:2] == ["go", "test"])
            self.assertIn("-run", go_test)
            self.assertIn("TestRuntime", go_test)
            self.assertEqual(go_test[-1], "./internal/desktop")


class DesktopCIGateContractTests(unittest.TestCase):
    def test_source_tree_hash_uses_the_sdk_file_size_framing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            file = root / "fixture.txt"
            file.write_bytes(b"abc")
            subprocess.run(["git", "add", "fixture.txt"], cwd=root, check=True)
            mode = stat.S_IMODE(os.lstat(file).st_mode)
            if os.name == "nt":
                mode &= ~0o111
            expected = hashlib.sha256(
                b"fixture.txt\x00" + ("%o" % mode).encode() + b"\x00" + b"3\x00abc\x00"
            ).hexdigest()
            self.assertEqual(BUILD.source_tree_hash(root), expected)

    def test_locked_source_tree_hashes_match_checked_out_bytes(self) -> None:
        source = json.loads((ROOT / "build/vivy-sources.lock.json").read_text(encoding="utf-8"))
        for name in ("vivy", "laputa"):
            with self.subTest(source=name):
                checkout = ROOT.parent / ("agent-vivy" if name == "vivy" else "laputa")
                self.assertEqual(BUILD.git(checkout, "rev-parse", "HEAD"), source["sources"][name]["commit"])
                self.assertEqual(BUILD.source_tree_hash(checkout), source["sources"][name]["treeSHA256"])

    def test_go_only_paths_trigger_push_and_pull_request(self) -> None:
        workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        required = {
            "cmd/diva/**", "internal/desktop/**", "internal/speech/**",
            "go.mod", "go.sum", "build/**", "scripts/**", "justfile",
            "agent-diva-gui/**", ".github/workflows/**",
        }
        for event in ("push", "pull_request"):
            match = re.search(rf"(?ms)^  {event}:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|^jobs:)", workflow)
            self.assertIsNotNone(match, f"missing {event} trigger")
            paths = set(re.findall(r"^\s*- ['\"]?([^'\"\n]+)", match.group(1), re.MULTILINE))
            self.assertTrue(required <= paths, f"{event} paths omit {sorted(required - paths)}")
            for changed in (
                "cmd/diva/main.go", "internal/desktop/runtime_service.go",
                "internal/speech/host.go", "build/vivy-sources.lock.json",
                "scripts/build-desktop.py", "justfile",
            ):
                self.assertTrue(any(fnmatch(changed, pattern) for pattern in paths), f"{event} misses {changed}")

    def test_aggregate_includes_native_race_bindings_boundary_and_seal(self) -> None:
        justfile = (ROOT / "justfile").read_text(encoding="utf-8")
        match = re.search(r"(?m)^ci:\s*(.*)$", justfile)
        self.assertIsNotNone(match, "justfile must define the aggregate ci recipe")
        dependencies = set(match.group(1).split())
        required = {
            "gui-test", "gui-build", "go-test", "desktop-bindings-check",
            "desktop-boundary-check", "desktop-seal-check", "desktop-contract-tests", "shell-bridge-test",
            "legacy-selftest", "transition-boundary-check",
        }
        self.assertTrue(required <= dependencies, f"ci omits {sorted(required - dependencies)}")
        gate = re.search(r"(?ms)^  desktop-ci-required:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|\Z)", (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8"))
        self.assertIsNotNone(gate)
        self.assertIn("legacy-boundary-gate", gate.group(1))
        self.assertIn("windows-shell-check", gate.group(1))

    def test_linux_and_windows_use_locked_sources(self) -> None:
        workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        source_lock = json.loads((ROOT / "build/vivy-sources.lock.json").read_text(encoding="utf-8"))
        for platform in ("linux-amd64", "windows-amd64"):
            self.assertIn(platform, workflow)
        for token in (
            "ProjectViVy/agent-vivy", "ProjectViVy/laputa", "source-lock",
            "check-lock", "--mode test", "packageManager", "wails3",
            "ref: ${{ steps.source-lock.outputs.vivy }}",
            "ref: ${{ steps.source-lock.outputs.laputa }}",
        ):
            self.assertIn(token, workflow, f"workflow is missing locked input check {token!r}")
        self.assertEqual(set(source_lock["tools"]["platforms"]), {"linux-amd64", "windows-amd64"})

    def test_target_locks_derive_from_single_source_owner(self) -> None:
        source = json.loads((ROOT / "build/vivy-sources.lock.json").read_text(encoding="utf-8"))
        linux = BUILD.derive_target_lock(source, "linux-amd64")
        windows = BUILD.derive_target_lock(source, "windows-amd64")
        self.assertEqual(linux["tools"]["goos"], "linux")
        self.assertEqual(linux["tools"]["goarch"], "amd64")
        self.assertEqual(linux["buildTags"], ["gtk3"])
        self.assertEqual(windows["tools"]["goos"], "windows")
        self.assertEqual(windows["tools"]["goarch"], "amd64")
        self.assertEqual(windows["buildTags"], [])
        self.assertEqual(linux["tools"]["sourceLockSHA256"], windows["tools"]["sourceLockSHA256"])
        self.assertEqual(
            linux["tools"]["sourceLockSHA256"],
            hashlib.sha256(BUILD.canonical_lock_bytes(source)).hexdigest(),
        )
        for field in ("sources", "recipe", "locks", "host"):
            self.assertEqual(linux[field], windows[field], f"platform derivation changed common {field}")
        for field in ("go", "wails", "node", "pnpm", "platforms"):
            self.assertEqual(linux["tools"][field], windows["tools"][field], f"target locks disagree on common tool {field}")
        self.assertEqual(BUILD.canonical_lock_bytes(source), BUILD.canonical_lock_bytes(copy.deepcopy(source)))
        self.assertTrue(BUILD.canonical_lock_bytes(source).endswith(b"\n"))
        self.assertNotIn(b": ", BUILD.canonical_lock_bytes(source))

        changed_source = copy.deepcopy(source)
        changed_source["sources"]["vivy"]["commit"] = "a" * 40
        changed_linux = BUILD.derive_target_lock(changed_source, "linux-amd64")
        changed_windows = BUILD.derive_target_lock(changed_source, "windows-amd64")
        self.assertNotEqual(linux["tools"]["sourceLockSHA256"], changed_linux["tools"]["sourceLockSHA256"])
        self.assertNotEqual(windows["tools"]["sourceLockSHA256"], changed_windows["tools"]["sourceLockSHA256"])

        edited = copy.deepcopy(linux)
        edited["sources"]["vivy"]["commit"] = "b" * 40
        self.assertNotEqual(BUILD.verify_derived_lock(source, edited, "linux-amd64"), [])

    def test_requested_native_target_mismatch_is_rejected(self) -> None:
        self.assertNotEqual(BUILD.validate_native_platform("linux-amd64", "windows", "amd64"), [])
        self.assertEqual(BUILD.validate_native_platform("linux-amd64", "linux", "amd64"), [])

    def test_source_lock_target_fields_must_match_native_toolchain(self) -> None:
        source = json.loads((ROOT / "build/vivy-sources.lock.json").read_text(encoding="utf-8"))
        self.assertEqual(BUILD.validate_locked_platform(source, "linux-amd64", "linux", "amd64", "1"), [])
        changed = copy.deepcopy(source)
        changed["tools"]["platforms"]["linux-amd64"]["goos"] = "windows"
        self.assertTrue(BUILD.validate_locked_platform(changed, "linux-amd64", "linux", "amd64", "1"))

    def test_check_lock_requires_explicit_native_platform(self) -> None:
        args = BUILD.argparse.Namespace(host_dir=str(ROOT), platform=None, derived_lock=None)
        with self.assertRaisesRegex(SystemExit, "requires --platform"):
            BUILD.mode_check_lock(args)

    def test_wails_cli_module_and_frontend_share_source_lock_pin(self) -> None:
        source = json.loads((ROOT / "build/vivy-sources.lock.json").read_text(encoding="utf-8"))
        self.assertEqual(BUILD.declared_tool_errors(source, ROOT), [])
        changed = copy.deepcopy(source)
        changed["tools"]["wails"] = "v3.0.0-beta.99"
        self.assertTrue(BUILD.declared_tool_errors(changed, ROOT))

    def test_packaged_artifact_retains_both_locks_in_checksum_set(self) -> None:
        source = json.loads((ROOT / "build/vivy-sources.lock.json").read_text(encoding="utf-8"))
        target = BUILD.derive_target_lock(source, "linux-amd64")
        with tempfile.TemporaryDirectory() as tmp:
            artifact = Path(tmp)
            (artifact / "checksums.sha256").write_text("abc  diva\n", encoding="utf-8")
            BUILD.write_artifact_locks(artifact, source, target)
            source_bytes = (artifact / "source-inputs.lock.json").read_bytes()
            target_bytes = (artifact / "input-lock.json").read_bytes()
            checksums = (artifact / "checksums.sha256").read_text(encoding="utf-8")
            self.assertEqual(source_bytes, BUILD.canonical_lock_bytes(source))
            self.assertEqual(target_bytes, BUILD.canonical_lock_bytes(target))
            self.assertIn(f"{hashlib.sha256(source_bytes).hexdigest()}  source-inputs.lock.json", checksums)
            self.assertIn(f"{hashlib.sha256(target_bytes).hexdigest()}  input-lock.json", checksums)
            self.assertIn("abc  diva", checksums)


if __name__ == "__main__":
    unittest.main()
