from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CHECKER_PATH = Path(__file__).with_name("check_wails_candidate.py")
SPEC = importlib.util.spec_from_file_location("check_wails_candidate", CHECKER_PATH)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)

BUILD_PATH = ROOT / "scripts" / "build-desktop.py"
BUILD_SPEC = importlib.util.spec_from_file_location("build_desktop", BUILD_PATH)
assert BUILD_SPEC is not None and BUILD_SPEC.loader is not None
BUILD = importlib.util.module_from_spec(BUILD_SPEC)
BUILD_SPEC.loader.exec_module(BUILD)

PLATFORMS = ("linux-amd64", "windows-amd64")
RELEASE_SHA = "a" * 40
GENERATION_ID = "c" * 64

ROWS = {
    "W5-T1-CANDIDATE": ("W-R2", "One sealed candidate pair: pinned sources, recipe, generation, binary hash recorded; inspect of final executable", ()),
    "W5-T2-FRESH-PROFILE": ("W-R9", "Clean Next profile boots to a sealed runtime without external bootstrap tooling", ()),
    "W5-T2-FIRST-TURN": ("W-R3", "First model turn on the sealed candidate with a configured model", ()),
    "W5-T2-GRANTS-CATALOG": ("W-R3/W-R6", "Sealed module-action catalog, grant checks, denied tool boundaries", ()),
    "W5-T3-CHAT-MATRIX": ("W-R3", "Text/images, presets+readback, approval accept/reject, active cancel, edit/regenerate, rewind conflict, goal entry, session select/reopen", (
        "text", "images", "preset-readback", "approval-accept", "approval-reject", "approval-cancel",
        "edit", "regenerate", "rewind-conflict", "goal-entry", "session-select", "session-reopen",
    )),
    "W5-T3-COGNITIVE": ("W-R3", "Cognitive context projection and controls; denied/busy/unsupported explicit", ()),
    "W5-T4-SINGLE-INSTANCE": ("W-R5", "Second process launch rejected while candidate runs", ()),
    "W5-T4-CRASH-RESTART": ("W-R5", "SIGKILL then relaunch on the same profile reopens the runtime and journal", ()),
    "W5-T4-CLEAN-QUIT": ("W-R5", "Normal quit path releases workspace and exits", ()),
    "W5-T4-HIDE-REOPEN": ("W-R5", "Hide/reopen keeps the same runtime, no replayed audio", ()),
    "W5-T4-WEBVIEW-RELOAD": ("W-R5", "WebView reload invalidates context and recovers without duplicate turns", ()),
    "W5-T4-EVENT-LOSS": ("W-R5", "Event queue overflow/loss, active-call timeout, shutdown deadline: recover without duplicate turns or fabricated deltas", (
        "queue-overflow", "active-call-timeout", "shutdown-deadline",
    )),
    "W5-T5-LOGS": ("W-R2", "Runtime/GUI logs, rotation, redaction, request correlation", ()),
    "W5-T5-VOICE-SCRIPTED": ("W-R4", "Voice input/output and provider error paths vs scripted providers", ()),
    "W5-T5-VOICE-REAL": ("W-R4", "Real configured SiliconFlow STT + SiliconFlow/MiniMax TTS", (
        "siliconflow-stt", "siliconflow-tts", "minimax-tts", "microphone", "playback",
    )),
    "W5-T5-VOICE-LIFECYCLE": ("W-R4/W-R5", "Hide/reload/session-switch cancel, stale generation, asset delete during use, unavailable keyring, replay silence", (
        "hide-cancel", "reload-cancel", "session-switch-cancel", "stale-generation-discard",
        "delete-asset-during-use", "keyring-unavailable", "replay-silence",
    )),
    "W5-WINDOWS-MATRIX": ("W-R2/W-R4/W-R5", "Whole matrix on Windows x64 (WebView2, Credential Manager, lifetime)", ()),
}


def digest(body: bytes) -> str:
    return hashlib.sha256(body).hexdigest()


def create_candidate(root: Path, *, include_installer: bool = False):
    source_lock = json.loads((ROOT / "build/vivy-sources.lock.json").read_text(encoding="utf-8"))
    source_bytes = BUILD.canonical_lock_bytes(source_lock)
    report: dict = {"schema": "diva.wails-candidate-acceptance/v2", "captured_at": "2026-10-09T00:00:00Z", "platforms": {}, "rows": []}
    report["promotion"] = {
        "w6_state": "approved",
        "archive_tags": {
            repository: {"ref": CHECKER.ARCHIVE_TAG, "commit": commit}
            for repository, commit in CHECKER.ARCHIVE_TAG_COMMITS.items()
        },
    }
    artifact_dirs: dict[str, Path] = {}

    for platform in PLATFORMS:
        platform_dir = root / platform
        frontend = platform_dir / "ui" / "dist"
        frontend.mkdir(parents=True)
        (frontend / "index.html").write_text("<!doctype html><html></html>\n", encoding="utf-8")
        binary_name = "diva.exe" if platform.startswith("windows") else "diva"
        binary = b"synthetic sealed binary " + platform.encode()
        (platform_dir / binary_name).write_bytes(binary)
        (platform_dir / "consumer.mod").write_text("module example.com/diva\n", encoding="utf-8")
        (platform_dir / "consumer.sum").write_text("", encoding="utf-8")
        source_raw = source_bytes
        target_lock = BUILD.derive_target_lock(source_lock, platform)
        target_raw = BUILD.canonical_lock_bytes(target_lock)
        (platform_dir / "source-inputs.lock.json").write_bytes(source_raw)
        (platform_dir / "input-lock.json").write_bytes(target_raw)

        host_tree = BUILD.source_tree_hash(ROOT)
        frontend_hash = CHECKER._hash_ui_asset_tree(frontend)
        goos = "windows" if platform.startswith("windows") else "linux"
        host_build = {
            "schema": "vivy.go-host/v1",
            "modulePath": "github.com/ProjectViVy/agent-diva",
            "package": "./cmd/diva",
            "source": {"repository": "https://github.com/ProjectViVy/agent-diva", "commit": RELEASE_SHA, "treeSHA256": host_tree},
            "assets": {"path": "agent-diva-gui/dist", "sha256": frontend_hash},
            "dependencyLockSHA256": digest(target_raw),
            "consumerModfileSHA256": digest((platform_dir / "consumer.mod").read_bytes()),
            "consumerSumSHA256": digest((platform_dir / "consumer.sum").read_bytes()),
            "tools": {
                "go": source_lock["tools"]["go"], "wails": source_lock["tools"]["wails"],
                "goos": goos, "goarch": "amd64", "cgo": True,
                "buildTags": target_lock["buildTags"],
            },
        }
        generation = {"generationId": GENERATION_ID, "hostBuild": host_build}
        (platform_dir / "generation.json").write_text(json.dumps(generation, sort_keys=True), encoding="utf-8")
        resolved_host = {"commit": RELEASE_SHA, "treeSHA256": host_tree, "dirty": []}
        vivy_pin = source_lock["sources"]["vivy"]
        laputa_pin = source_lock["sources"]["laputa"]
        build_report = {
            "schema": "vivy.go-host-report/v1", "release": True, "generationId": GENERATION_ID,
            "resolved": {
                "host": resolved_host,
                "vivy": {"commit": vivy_pin["commit"], "treeSHA256": vivy_pin["treeSHA256"], "dirty": []},
                "laputa": {"commit": laputa_pin["commit"], "treeSHA256": laputa_pin["treeSHA256"], "dirty": []},
                "modulePath": host_build["modulePath"], "package": host_build["package"],
                "consumerModfileSHA256": host_build["consumerModfileSHA256"],
                "consumerSumSHA256": host_build["consumerSumSHA256"],
                "dependencyLockSHA256": host_build["dependencyLockSHA256"],
                "tools": host_build["tools"],
            },
        }
        (platform_dir / "build-report.json").write_text(json.dumps(build_report, sort_keys=True), encoding="utf-8")
        inspection = {"binary": str(platform_dir / binary_name), "manifest": {"generationId": GENERATION_ID}}
        (platform_dir / "native-inspection.json").write_text(json.dumps(inspection, sort_keys=True), encoding="utf-8")
        if include_installer and platform.startswith("windows"):
            (platform_dir / "diva-installer.exe").write_bytes(b"synthetic installer")

        checksum_paths = [
            binary_name, "generation.json", "consumer.mod", "consumer.sum",
            "source-inputs.lock.json", "input-lock.json", "ui/dist/index.html",
        ]
        checksum_body = "".join(f"{digest((platform_dir / rel).read_bytes())}  {rel}\n" for rel in sorted(checksum_paths))
        (platform_dir / "checksums.sha256").write_text(checksum_body, encoding="utf-8")
        artifact_hashes = {
            path.relative_to(platform_dir).as_posix(): digest(path.read_bytes())
            for path in sorted(platform_dir.rglob("*")) if path.is_file()
        }
        target = target_lock["tools"]["platforms"][platform]
        platform_sources = {
            "host": {"commit": RELEASE_SHA, "treeSHA256": host_tree},
            "vivy": {"commit": vivy_pin["commit"], "treeSHA256": vivy_pin["treeSHA256"]},
            "laputa": {"commit": laputa_pin["commit"], "treeSHA256": laputa_pin["treeSHA256"]},
            "inofy": source_lock["sources"]["inofy"],
        }
        report["platforms"][platform] = {
            "sources": platform_sources,
            "recipe": copy.deepcopy(source_lock["recipe"]),
            "locks": {
                "sourceInputsSHA256": digest(source_raw), "inputLockSHA256": digest(target_raw),
                "goModSHA256": source_lock["locks"]["go.mod"],
                "goSumSHA256": source_lock["locks"]["go.sum"],
                "pnpmLockSHA256": source_lock["locks"]["pnpm-lock.yaml"],
                "consumerModfileSHA256": host_build["consumerModfileSHA256"],
                "consumerSumSHA256": host_build["consumerSumSHA256"],
            },
            "generation_id": GENERATION_ID,
            "frontend_sha256": frontend_hash,
            "tools": {
                "go": source_lock["tools"]["go"], "wails": source_lock["tools"]["wails"],
                "node": source_lock["tools"]["node"], "pnpm": source_lock["tools"]["pnpm"],
                "goos": target["goos"], "goarch": target["goarch"], "cgo": target["cgo"],
                "build_tags": target["buildTags"],
            },
            "artifacts": artifact_hashes,
        }
        artifact_dirs[platform] = platform_dir

    for row_id, (requirement, scenario, required_subcases) in ROWS.items():
        results = []
        for platform in PLATFORMS:
            evidence = [{"path": "generation.json", "sha256": report["platforms"][platform]["artifacts"]["generation.json"]}]
            results.append({
                "platform": platform, "outcome": "passed", "evidence": evidence,
                "subcases": [
                    {"id": subcase, "outcome": "passed", "evidence": evidence}
                    for subcase in required_subcases
                ],
            })
        report["rows"].append({"id": row_id, "requirement": requirement, "scenario": scenario, "results": results})
    return report, artifact_dirs


class WailsCandidateStrictTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.doc, self.artifacts = create_candidate(self.root)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def validate(self, doc=None, *, require_all_passed: bool = True, artifacts=None, release_sha: str = RELEASE_SHA):
        return CHECKER.validate_candidate(doc or self.doc, artifacts or self.artifacts, release_sha, require_all_passed)

    def test_complete_synthetic_candidate_passes(self) -> None:
        self.assertEqual(self.validate(), [])

    def test_missing_required_row_rejected(self) -> None:
        doc = copy.deepcopy(self.doc)
        doc["rows"].pop()
        self.assertTrue(self.validate(doc))

    def test_unknown_row_cannot_substitute(self) -> None:
        doc = copy.deepcopy(self.doc)
        doc["rows"][0]["id"] = "W5-UNKNOWN"
        self.assertTrue(self.validate(doc))

    def test_missing_required_subcase_rejected(self) -> None:
        doc = copy.deepcopy(self.doc)
        chat = next(row for row in doc["rows"] if row["id"] == "W5-T3-CHAT-MATRIX")
        chat["results"][0]["subcases"].pop()
        self.assertTrue(self.validate(doc))

    def test_pending_or_failed_platform_blocks_publication(self) -> None:
        pending = copy.deepcopy(self.doc)
        row = pending["rows"][0]
        result = row["results"][1]
        result.update(outcome="pending", owner="owner", evidence=[])
        self.assertTrue(self.validate(pending, require_all_passed=True))
        failed = copy.deepcopy(self.doc)
        failed["rows"][0]["results"][0]["outcome"] = "failed"
        self.assertTrue(self.validate(failed, require_all_passed=False))

    def test_strict_requires_both_platforms_reports_and_binaries(self) -> None:
        missing_platform = copy.deepcopy(self.doc)
        del missing_platform["platforms"]["windows-amd64"]
        self.assertTrue(self.validate(missing_platform))
        missing_report_dir = dict(self.artifacts)
        missing_report_dir["linux-amd64"] = self.root / "absent-linux"
        self.assertTrue(self.validate(self.doc, artifacts=missing_report_dir))
        missing_binary_dir = copy.deepcopy(self.artifacts)
        (missing_binary_dir["windows-amd64"] / "diva.exe").unlink()
        self.assertTrue(self.validate(self.doc))

    def test_full_sha_not_prefix(self) -> None:
        doc = copy.deepcopy(self.doc)
        doc["platforms"]["linux-amd64"]["sources"]["host"]["commit"] = RELEASE_SHA[:12]
        self.assertTrue(self.validate(doc))

    def test_dirty_vivy_or_laputa_is_rejected(self) -> None:
        for source in ("vivy", "laputa"):
            with self.subTest(source=source):
                doc = copy.deepcopy(self.doc)
                report_path = self.artifacts["linux-amd64"] / "build-report.json"
                build_report = json.loads(report_path.read_text(encoding="utf-8"))
                build_report["resolved"][source]["dirty"] = ["unexpected.go"]
                report_path.write_text(json.dumps(build_report, sort_keys=True), encoding="utf-8")
                doc["platforms"]["linux-amd64"]["artifacts"]["build-report.json"] = digest(report_path.read_bytes())
                self.assertTrue(self.validate(doc))
                report_path.write_text(json.dumps(build_report | {"resolved": build_report["resolved"]}, sort_keys=True), encoding="utf-8")
                # Rebuild the pristine fixture before checking the next source.
                if source == "vivy":
                    self.temp.cleanup()
                    self.temp = tempfile.TemporaryDirectory()
                    self.root = Path(self.temp.name)
                    self.doc, self.artifacts = create_candidate(self.root)

    def test_recipe_generation_frontend_lock_tool_and_platform_mismatches(self) -> None:
        mutations = [
            lambda doc: doc["platforms"]["linux-amd64"]["recipe"].update(sha256="0" * 64),
            lambda doc: doc["platforms"]["linux-amd64"].update(generation_id="0" * 64),
            lambda doc: doc["platforms"]["linux-amd64"].update(frontend_sha256="0" * 64),
            lambda doc: doc["platforms"]["linux-amd64"]["locks"].update(inputLockSHA256="0" * 64),
            lambda doc: doc["platforms"]["linux-amd64"]["tools"].update(wails="v3.0.0-beta.99"),
            lambda doc: doc["platforms"]["linux-amd64"]["tools"].update(goos="windows"),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                doc = copy.deepcopy(self.doc)
                mutate(doc)
                self.assertTrue(self.validate(doc))

    def test_binary_and_installer_tamper_rejected(self) -> None:
        doc, artifacts = create_candidate(self.root / "tamper", include_installer=True)
        binary = artifacts["linux-amd64"] / "diva"
        binary.write_bytes(b"tampered binary")
        self.assertTrue(self.validate(doc, artifacts=artifacts))
        installer = artifacts["windows-amd64"] / "diva-installer.exe"
        installer.write_bytes(b"tampered installer")
        self.assertTrue(self.validate(doc, artifacts=artifacts))

    def test_stale_evidence_hash_rejected(self) -> None:
        doc = copy.deepcopy(self.doc)
        doc["rows"][0]["results"][0]["evidence"][0]["sha256"] = "0" * 64
        self.assertTrue(self.validate(doc))

    def test_historical_v1_cannot_promote(self) -> None:
        historical = json.loads((ROOT / "docs/plans/diva-next/fixtures/wails-candidate-acceptance.json").read_text(encoding="utf-8"))
        self.assertTrue(CHECKER.validate_candidate(historical, {}, RELEASE_SHA, True))

    def test_archive_tags_and_selected_w6_state_are_required_for_promotion(self) -> None:
        pending_w6 = copy.deepcopy(self.doc)
        pending_w6["promotion"]["w6_state"] = "pending"
        self.assertTrue(self.validate(pending_w6))
        wrong_archive = copy.deepcopy(self.doc)
        wrong_archive["promotion"]["archive_tags"]["agent-vivy"]["commit"] = "d" * 40
        self.assertTrue(self.validate(wrong_archive))


if __name__ == "__main__":
    unittest.main()
