#!/usr/bin/env python3
"""Validate historical W5 v1 reports or strict, byte-bound v2 candidates."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
PLATFORMS = ("linux-amd64", "windows-amd64")
ARCHIVE_TAG = "archive/tauri-cabi-20261004"
ARCHIVE_TAG_COMMITS = {
    "agent-diva": "5444795a2d9db31e158c2cf009d64697e6289e50",
    "agent-vivy": "fc559e6b03ce4e65c0099b9745855dccc4fb067e",
}
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
GIT_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
ZERO_DIGEST = "0" * 64
HASHED_FILENAME_RE = re.compile(r"-[A-Za-z0-9_-]{8}(\.[^./]+)$")
ASSET_HASH_RE = re.compile(rb'(:\s*")[0-9a-f]{64}(")')

# Frozen W5 acceptance vocabulary. Changes require an explicit acceptance-plan
# revision; callers cannot substitute a new row or scenario to make a pass.
REQUIRED_ROWS: dict[str, tuple[str, str, tuple[str, ...]]] = {
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


def _build_module():
    path = ROOT / "scripts" / "build-desktop.py"
    spec = importlib.util.spec_from_file_location("diva_build_desktop", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load build helper {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _error(errors: list[str], where: str, message: str) -> None:
    errors.append(f"{where}: {message}")


def _valid_sha(value: Any, pattern: re.Pattern[str]) -> bool:
    return isinstance(value, str) and pattern.fullmatch(value) is not None


def _hash_ui_asset_tree(root: Path) -> str:
    """Port sdk/internal.hashUIArtifactTree for packaged ui/dist bytes."""
    files: list[Path] = []
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError(f"UI artifact contains symbolic link {path.relative_to(root)}")
        if path.is_file():
            files.append(path)
        elif not path.is_dir():
            raise ValueError(f"UI artifact contains non-regular entry {path.relative_to(root)}")
    files.sort(key=lambda path: path.as_posix())

    references: dict[str, str] = {}
    canonical_paths: dict[Path, str] = {}
    for path in files:
        rel = path.relative_to(root).as_posix()
        canonical = HASHED_FILENAME_RE.sub("-" + ZERO_DIGEST[:8] + r"\1", rel)
        canonical_paths[path] = canonical
        if canonical != rel:
            references[rel] = canonical
            references[path.name] = Path(canonical).name

    h = hashlib.sha256()
    for path in files:
        body = path.read_bytes()
        for raw in sorted(references, key=lambda item: (-len(item), item)):
            body = body.replace(raw.encode(), references[raw].encode())
        body = _mask_asset_hashes(body)
        h.update(canonical_paths[path].encode())
        h.update(b"\x00")
        h.update(body)
        h.update(b"\x00")
    return h.hexdigest()


def _mask_asset_hashes(body: bytes) -> bytes:
    key_start = body.find(b"assetHashes")
    if key_start < 0:
        return body
    open_offset = body.find(b"{", key_start + len(b"assetHashes"))
    if open_offset < 0:
        return body
    depth = 0
    in_string = False
    escaped = False
    close = -1
    for index in range(open_offset, len(body)):
        char = body[index]
        if in_string:
            if escaped:
                escaped = False
            elif char == 0x5C:
                escaped = True
            elif char == 0x22:
                in_string = False
            continue
        if char == 0x22:
            in_string = True
        elif char == 0x7B:
            depth += 1
        elif char == 0x7D:
            depth -= 1
            if depth == 0:
                close = index
                break
    if close < 0:
        return body
    section = ASSET_HASH_RE.sub(rb"\g<1>" + ZERO_DIGEST.encode() + rb"\g<2>", body[open_offset : close + 1])
    return body[:open_offset] + section + body[close + 1 :]


def _artifact_files(artifact_dir: Path, errors: list[str], platform: str) -> dict[str, str]:
    found: dict[str, str] = {}
    if not artifact_dir.is_dir():
        _error(errors, platform, f"artifact directory missing: {artifact_dir}")
        return found
    for path in sorted(artifact_dir.rglob("*")):
        rel = path.relative_to(artifact_dir).as_posix()
        if path.is_symlink():
            _error(errors, platform, f"artifact contains symbolic link {rel}")
        elif path.is_file():
            found[rel] = digest(path.read_bytes())
        elif not path.is_dir():
            _error(errors, platform, f"artifact contains non-regular entry {rel}")
    return found


def validate_platform(platform: str, candidate: dict[str, Any], artifact_dir: Path) -> list[str]:
    errors: list[str] = []
    where = platform
    try:
        source_lock_path = ROOT / "build" / "vivy-sources.lock.json"
        source_lock = json.loads(source_lock_path.read_text(encoding="utf-8"))
        build = _build_module()
    except Exception as exc:
        return [f"{where}: cannot load canonical source lock/build helper: {exc}"]

    declared_files = candidate.get("artifacts")
    if not isinstance(declared_files, dict):
        _error(errors, where, "artifacts must map retained relative paths to SHA-256")
        declared_files = {}
    for rel, sha in declared_files.items():
        path = Path(rel)
        if path.is_absolute() or ".." in path.parts or not _valid_sha(sha, SHA256_RE):
            _error(errors, where, f"invalid retained artifact entry {rel!r}")

    actual_files = _artifact_files(artifact_dir, errors, platform)
    for rel, actual_sha in actual_files.items():
        expected_sha = declared_files.get(rel)
        if expected_sha != actual_sha:
            _error(errors, where, f"artifact hash mismatch or unrecorded file: {rel}")
    for rel in declared_files.keys() - actual_files.keys():
        _error(errors, where, f"retained artifact missing: {rel}")

    required_names = {
        "build-report.json", "generation.json", "checksums.sha256", "native-inspection.json",
        "consumer.mod", "consumer.sum", "source-inputs.lock.json", "input-lock.json",
    }
    binary = "diva.exe" if platform == "windows-amd64" else "diva"
    required_names.add(binary)
    for rel in sorted(required_names):
        if rel not in actual_files:
            _error(errors, where, f"required candidate file missing: {rel}")

    try:
        source_raw = build.canonical_lock_bytes(source_lock)
        target_lock = build.derive_target_lock(source_lock, platform)
        target_raw = build.canonical_lock_bytes(target_lock)
        if (artifact_dir / "source-inputs.lock.json").read_bytes() != source_raw:
            _error(errors, where, "retained source lock bytes differ from canonical lock")
        if (artifact_dir / "input-lock.json").read_bytes() != target_raw:
            _error(errors, where, "retained target lock differs from deterministic derivation")
    except Exception as exc:
        _error(errors, where, f"cannot verify canonical and derived locks: {exc}")
        return errors

    try:
        report = json.loads((artifact_dir / "build-report.json").read_text(encoding="utf-8"))
        generation = json.loads((artifact_dir / "generation.json").read_text(encoding="utf-8"))
        inspection = json.loads((artifact_dir / "native-inspection.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        _error(errors, where, f"cannot read retained build/generation/inspection report: {exc}")
        return errors

    if not isinstance(report, dict) or not isinstance(generation, dict) or not isinstance(inspection, dict):
        _error(errors, where, "retained build/generation/inspection documents must be JSON objects")
        return errors
    if report.get("schema") != "vivy.go-host-report/v1" or report.get("release") is not True:
        _error(errors, where, "build report is not a release vivy.go-host report")
    resolved = report.get("resolved") if isinstance(report.get("resolved"), dict) else {}
    host = resolved.get("host") if isinstance(resolved.get("host"), dict) else {}
    vivy = resolved.get("vivy") if isinstance(resolved.get("vivy"), dict) else {}
    laputa = resolved.get("laputa") if isinstance(resolved.get("laputa"), dict) else {}
    host_build = generation.get("hostBuild") if isinstance(generation.get("hostBuild"), dict) else {}
    sources = candidate.get("sources") if isinstance(candidate.get("sources"), dict) else {}
    if not sources:
        _error(errors, where, "candidate sources object is missing")
    for source_name, actual in (("host", host), ("vivy", vivy), ("laputa", laputa)):
        declared = sources.get(source_name) or {}
        commit = actual.get("commit", "")
        tree_sha = actual.get("treeSHA256", "")
        if not _valid_sha(declared.get("commit"), GIT_SHA_RE):
            _error(errors, where, f"{source_name} commit must be a full 40-character SHA")
        if not _valid_sha(declared.get("treeSHA256"), SHA256_RE):
            _error(errors, where, f"{source_name} treeSHA256 must be a full SHA-256")
        if commit != declared.get("commit") or tree_sha != declared.get("treeSHA256"):
            _error(errors, where, f"{source_name} identity differs from build report")
        if actual.get("dirty"):
            _error(errors, where, f"{source_name} source is dirty: {actual.get('dirty')}")
        source_root = {
            "host": ROOT,
            "vivy": ROOT.parent / "agent-vivy",
            "laputa": ROOT.parent / "laputa",
        }[source_name]
        try:
            actual_tree = build.source_tree_hash(source_root)
            if actual_tree != tree_sha:
                _error(errors, where, f"{source_name} source tree differs from checked-out bytes")
        except Exception as exc:
            _error(errors, where, f"cannot hash checked-out {source_name} source: {exc}")
    if not GIT_SHA_RE.fullmatch(str(host.get("commit", ""))):
        _error(errors, where, "build report host commit must be a full 40-character SHA")
    declared_host = sources.get("host") if isinstance(sources.get("host"), dict) else {}
    if declared_host.get("commit") != host.get("commit"):
        _error(errors, where, "host commit differs from build report")
    expected_vivy = source_lock.get("sources", {}).get("vivy", {})
    expected_laputa = source_lock.get("sources", {}).get("laputa", {})
    for key, expected in (("vivy", expected_vivy), ("laputa", expected_laputa)):
        declared = sources.get(key) or {}
        if declared.get("commit") != expected.get("commit") or declared.get("treeSHA256") != expected.get("treeSHA256"):
            _error(errors, where, f"{key} source does not match canonical source lock")
    inofy = sources.get("inofy") or {}
    expected_inofy = source_lock.get("sources", {}).get("inofy", {})
    if not GIT_SHA_RE.fullmatch(str(inofy.get("commit", ""))):
        _error(errors, where, "inofy commit must be a full 40-character SHA")
    if inofy != expected_inofy:
        _error(errors, where, "inofy source identity differs from canonical source lock")

    host_sha = host.get("commit", "")
    host_tree = host.get("treeSHA256", "")
    if host_build.get("schema") != "vivy.go-host/v1":
        _error(errors, where, "generation hostBuild schema mismatch")
    sealed_source = host_build.get("source") if isinstance(host_build.get("source"), dict) else {}
    if sealed_source.get("commit") != host_sha or sealed_source.get("treeSHA256") != host_tree:
        _error(errors, where, "generation hostBuild source does not match build report")
    if host_build.get("modulePath") != source_lock.get("host", {}).get("modulePath") or host_build.get("package") != "./cmd/diva":
        _error(errors, where, "generation hostBuild module/package mismatch")
    if report.get("generationId") != generation.get("generationId"):
        _error(errors, where, "generation ID differs between build report and manifest")
    if candidate.get("generation_id") != generation.get("generationId"):
        _error(errors, where, "candidate generation_id differs from retained manifest")

    try:
        frontend_path = artifact_dir / "ui" / "dist"
        frontend_sha = _hash_ui_asset_tree(frontend_path)
    except Exception as exc:
        _error(errors, where, f"cannot verify packaged frontend tree: {exc}")
        frontend_sha = ""
    if candidate.get("frontend_sha256") != frontend_sha:
        _error(errors, where, "frontend_sha256 differs from packaged frontend bytes")
    sealed_assets = host_build.get("assets") if isinstance(host_build.get("assets"), dict) else {}
    if sealed_assets.get("sha256") != frontend_sha:
        _error(errors, where, "generation hostBuild frontend digest mismatch")

    try:
        recipe = source_lock.get("recipe", {})
        if candidate.get("recipe") != recipe:
            _error(errors, where, "recipe identity differs from canonical source lock")
        vivy_root = ROOT.parent / "agent-vivy"
        if not vivy_root.is_dir():
            raise FileNotFoundError(f"locked VIVY checkout is unavailable at {vivy_root}")
        recipe_body = (vivy_root / recipe["path"]).read_bytes()
        if digest(recipe_body) != recipe.get("sha256"):
            _error(errors, where, "canonical recipe bytes do not match pinned digest")
    except (KeyError, OSError, TypeError) as exc:
        _error(errors, where, f"cannot verify canonical recipe: {exc}")

    target = source_lock.get("tools", {}).get("platforms", {}).get(platform, {})
    tools = candidate.get("tools") or {}
    expected_tools = {
        "go": source_lock.get("tools", {}).get("go"),
        "wails": source_lock.get("tools", {}).get("wails"),
        "node": source_lock.get("tools", {}).get("node"),
        "pnpm": source_lock.get("tools", {}).get("pnpm"),
        "goos": target.get("goos"), "goarch": target.get("goarch"),
        "cgo": target.get("cgo"), "build_tags": target.get("buildTags", []),
    }
    if tools != expected_tools:
        _error(errors, where, "tool/platform metadata differs from canonical lock")
    build_tools = host_build.get("tools") if isinstance(host_build.get("tools"), dict) else {}
    for report_key, expected_key in (("go", "go"), ("wails", "wails"), ("goos", "goos"), ("goarch", "goarch"), ("cgo", "cgo"), ("buildTags", "build_tags")):
        if build_tools.get(report_key) != expected_tools.get(expected_key):
            _error(errors, where, f"generation hostBuild tool/platform {report_key} mismatch")
    if resolved.get("tools") != build_tools:
        _error(errors, where, "build report tools differ from sealed hostBuild tools")

    locks = candidate.get("locks") or {}
    try:
        consumer_mod_sha = digest((artifact_dir / "consumer.mod").read_bytes())
        consumer_sum_sha = digest((artifact_dir / "consumer.sum").read_bytes())
    except OSError as exc:
        _error(errors, where, f"cannot read retained consumer module locks: {exc}")
        consumer_mod_sha = consumer_sum_sha = ""
    expected_locks = {
        "sourceInputsSHA256": digest(source_raw),
        "inputLockSHA256": digest(target_raw),
        "goModSHA256": source_lock.get("locks", {}).get("go.mod"),
        "goSumSHA256": source_lock.get("locks", {}).get("go.sum"),
        "pnpmLockSHA256": source_lock.get("locks", {}).get("pnpm-lock.yaml"),
        "consumerModfileSHA256": consumer_mod_sha,
        "consumerSumSHA256": consumer_sum_sha,
    }
    if locks != expected_locks:
        _error(errors, where, "lock hashes differ from retained/canonical lock bytes")
    for file_name, report_key in (("consumer.mod", "consumerModfileSHA256"), ("consumer.sum", "consumerSumSHA256")):
        if resolved.get(report_key) != expected_locks[report_key] or host_build.get(report_key) != expected_locks[report_key]:
            _error(errors, where, f"{file_name} hash differs from build report or sealed manifest")
    if resolved.get("dependencyLockSHA256") != digest(target_raw) or host_build.get("dependencyLockSHA256") != digest(target_raw):
        _error(errors, where, "derived dependency lock hash differs from build report or sealed manifest")

    inspection_manifest = inspection.get("manifest") if isinstance(inspection.get("manifest"), dict) else {}
    if inspection_manifest.get("generationId") != generation.get("generationId"):
        _error(errors, where, "native inspection generation does not match retained manifest")
    inspected_binary = inspection.get("binary")
    if not isinstance(inspected_binary, str) or Path(inspected_binary).name != binary:
        _error(errors, where, "native inspection did not identify the candidate executable")
    binary_path = artifact_dir / binary
    if binary_path.is_file() and digest(binary_path.read_bytes()) != actual_files.get(binary):
        _error(errors, where, "candidate executable hash is unavailable or inconsistent")
    return errors


def validate_rows(rows: list[dict[str, Any]], require_all_passed: bool) -> list[str]:
    errors: list[str] = []
    if not isinstance(rows, list):
        return ["rows must be an array"]
    seen: set[str] = set()
    for row in rows:
        if not isinstance(row, dict):
            errors.append("row entry must be an object")
            continue
        row_id = row.get("id")
        if not isinstance(row_id, str) or row_id not in REQUIRED_ROWS:
            errors.append(f"unknown acceptance row {row_id!r}")
            continue
        if row_id in seen:
            errors.append(f"duplicate acceptance row {row_id}")
        seen.add(row_id)
        requirement, scenario, subcases = REQUIRED_ROWS[row_id]
        if row.get("requirement") != requirement:
            errors.append(f"{row_id}: requirement differs from canonical mapping")
        if row.get("scenario") != scenario:
            errors.append(f"{row_id}: scenario differs from canonical mapping")
        results = row.get("results")
        if not isinstance(results, list):
            errors.append(f"{row_id}: results must be an array")
            continue
        result_platforms: set[str] = set()
        for result in results:
            if not isinstance(result, dict):
                errors.append(f"{row_id}: platform result must be an object")
                continue
            platform = result.get("platform")
            if platform not in PLATFORMS:
                errors.append(f"{row_id}: unknown platform result {platform!r}")
                continue
            if platform in result_platforms:
                errors.append(f"{row_id}: duplicate result for {platform}")
            result_platforms.add(platform)
            outcome = result.get("outcome")
            owner = result.get("owner")
            if outcome not in ("passed", "failed", "pending"):
                errors.append(f"{row_id}/{platform}: invalid outcome {outcome!r}")
            if outcome == "failed":
                errors.append(f"{row_id}/{platform}: failed acceptance blocks promotion")
            if outcome == "pending":
                if not owner:
                    errors.append(f"{row_id}/{platform}: pending acceptance requires owner")
                if require_all_passed:
                    errors.append(f"{row_id}/{platform}: pending acceptance blocks promotion")
            evidence = result.get("evidence")
            if not isinstance(evidence, list):
                errors.append(f"{row_id}/{platform}: evidence must be an array")
            elif outcome == "passed" and not evidence:
                errors.append(f"{row_id}/{platform}: passed result requires hashed evidence")
            subcase_entries = result.get("subcases", [])
            if not isinstance(subcase_entries, list):
                errors.append(f"{row_id}/{platform}: subcases must be an array")
                continue
            subcase_map: dict[str, dict[str, Any]] = {}
            for subcase in subcase_entries:
                if not isinstance(subcase, dict) or not isinstance(subcase.get("id"), str):
                    errors.append(f"{row_id}/{platform}: malformed subcase")
                    continue
                subcase_id = subcase["id"]
                if subcase_id in subcase_map:
                    errors.append(f"{row_id}/{platform}: duplicate subcase {subcase_id}")
                subcase_map[subcase_id] = subcase
            if set(subcase_map) != set(subcases):
                errors.append(f"{row_id}/{platform}: subcases must equal {list(subcases)}")
            for subcase_id, subcase in subcase_map.items():
                sub_outcome = subcase.get("outcome")
                if sub_outcome not in ("passed", "failed", "pending"):
                    errors.append(f"{row_id}/{platform}/{subcase_id}: invalid outcome {sub_outcome!r}")
                if sub_outcome == "failed":
                    errors.append(f"{row_id}/{platform}/{subcase_id}: failed subcase blocks promotion")
                if sub_outcome == "pending" and require_all_passed:
                    errors.append(f"{row_id}/{platform}/{subcase_id}: pending subcase blocks promotion")
                sub_evidence = subcase.get("evidence")
                if not isinstance(sub_evidence, list):
                    errors.append(f"{row_id}/{platform}/{subcase_id}: evidence must be an array")
                elif sub_outcome == "passed" and not sub_evidence:
                    errors.append(f"{row_id}/{platform}/{subcase_id}: passed subcase requires hashed evidence")
        if result_platforms != set(PLATFORMS):
            errors.append(f"{row_id}: results must include exactly {list(PLATFORMS)}")
    for required_id in REQUIRED_ROWS.keys() - seen:
        errors.append(f"missing required acceptance row {required_id}")
    return errors


def _validate_evidence(doc: dict[str, Any], artifacts: dict[str, Path]) -> list[str]:
    errors: list[str] = []
    for row in doc.get("rows", []) if isinstance(doc.get("rows"), list) else []:
        if not isinstance(row, dict):
            continue
        row_id = row.get("id", "<unknown>")
        for result in row.get("results", []) if isinstance(row.get("results"), list) else []:
            if not isinstance(result, dict) or result.get("platform") not in artifacts:
                continue
            platform = result["platform"]
            subcase_evidence: list[Any] = []
            subcases = result.get("subcases", [])
            if isinstance(subcases, list):
                for subcase in subcases:
                    if isinstance(subcase, dict) and isinstance(subcase.get("evidence", []), list):
                        subcase_evidence.extend(subcase.get("evidence", []))
            for label, records in (("result", result.get("evidence", [])), ("subcase", subcase_evidence)):
                if not isinstance(records, list):
                    continue
                for entry in records:
                    if not isinstance(entry, dict):
                        errors.append(f"{row_id}/{platform}: malformed {label} evidence")
                        continue
                    rel = entry.get("path")
                    expected = entry.get("sha256")
                    rel_path = Path(rel) if isinstance(rel, str) else Path("/")
                    if rel_path.is_absolute() or ".." in rel_path.parts or not _valid_sha(expected, SHA256_RE):
                        errors.append(f"{row_id}/{platform}: unsafe or invalid {label} evidence path/hash")
                        continue
                    path = artifacts[platform] / rel_path
                    try:
                        if not path.is_file() or digest(path.read_bytes()) != expected:
                            errors.append(f"{row_id}/{platform}: stale {label} evidence hash for {rel}")
                    except OSError:
                        errors.append(f"{row_id}/{platform}: missing {label} evidence file {rel}")
    return errors


def validate_candidate(
    doc: dict[str, Any], artifacts: dict[str, Path], release_source_sha: str | None,
    require_all_passed: bool,
) -> list[str]:
    errors: list[str] = []
    if not isinstance(doc, dict) or doc.get("schema") != "diva.wails-candidate-acceptance/v2":
        return ["strict promotion requires schema diva.wails-candidate-acceptance/v2; historical v1 cannot promote"]
    if not isinstance(doc.get("captured_at"), str) or not doc.get("captured_at"):
        errors.append("captured_at is required")
    promotion = doc.get("promotion") if isinstance(doc.get("promotion"), dict) else {}
    if promotion.get("w6_state") != "approved":
        errors.append("selected W6 state is not approved")
    archive_tags = promotion.get("archive_tags") if isinstance(promotion.get("archive_tags"), dict) else {}
    if set(archive_tags) != set(ARCHIVE_TAG_COMMITS):
        errors.append("promotion must identify both paired archive tags")
    for repository, expected_commit in ARCHIVE_TAG_COMMITS.items():
        record = archive_tags.get(repository) if isinstance(archive_tags.get(repository), dict) else {}
        if record.get("ref") != ARCHIVE_TAG or record.get("commit") != expected_commit:
            errors.append(f"{repository} archive tag must be {ARCHIVE_TAG} at {expected_commit}")
    if not _valid_sha(release_source_sha, GIT_SHA_RE):
        errors.append("release source SHA must be a full 40-character commit SHA")
    platforms = doc.get("platforms")
    if not isinstance(platforms, dict) or set(platforms) != set(PLATFORMS):
        errors.append(f"platforms must include exactly {list(PLATFORMS)}")
        platforms = platforms if isinstance(platforms, dict) else {}
    if set(artifacts) != set(PLATFORMS):
        errors.append(f"artifact root must include exactly {list(PLATFORMS)}")
    for platform in PLATFORMS:
        if platform not in platforms or platform not in artifacts:
            errors.append(f"missing candidate report or retained artifacts for {platform}")
            continue
        candidate = platforms[platform]
        if not isinstance(candidate, dict):
            errors.append(f"{platform}: candidate metadata must be an object")
            continue
        sources = candidate.get("sources") if isinstance(candidate.get("sources"), dict) else {}
        declared_host = sources.get("host") if isinstance(sources.get("host"), dict) else {}
        if declared_host.get("commit") != release_source_sha:
            errors.append(f"{platform}: host commit does not equal selected release tag commit")
        errors.extend(validate_platform(platform, candidate, Path(artifacts[platform])))
    errors.extend(validate_rows(doc.get("rows"), require_all_passed))
    errors.extend(_validate_evidence(doc, artifacts))
    return errors


def _legacy_validate(args: argparse.Namespace, doc: dict[str, Any]) -> int:
    problems: list[str] = []
    if doc.get("schema") != "diva.wails-candidate-acceptance/v1":
        problems.append("schema mismatch")
    candidate = doc.get("candidate") or {}
    pins = candidate.get("pins") or {}
    for key in ("agent_diva", "agent_vivy", "laputa", "wails", "go"):
        if key not in pins:
            problems.append(f"missing pin {key}")
    if not candidate.get("binary_sha256"):
        problems.append("missing binary_sha256")
    if not candidate.get("generation_id"):
        problems.append("missing generation_id")
    rows = doc.get("rows") or []
    if not rows:
        problems.append("no rows")
    seen: set[str] = set()
    pending: list[str] = []
    failed: list[str] = []
    for row in rows:
        row_id = row.get("id")
        if not row_id or row_id in seen:
            problems.append(f"bad/dup row id {row_id!r}")
        seen.add(row_id)
        if not str(row.get("requirement", "")).startswith("W-R"):
            problems.append(f"{row_id}: bad requirement {row.get('requirement')!r}")
        if not row.get("scenario"):
            problems.append(f"{row_id}: empty scenario")
        outcome = row.get("outcome")
        if outcome not in ("passed", "failed", "pending"):
            problems.append(f"{row_id}: bad outcome {outcome!r}")
        if not row.get("evidence"):
            problems.append(f"{row_id}: empty evidence")
        if outcome == "failed":
            failed.append(row_id)
        if outcome == "pending":
            pending.append(row_id)
            if not row.get("owner"):
                problems.append(f"{row_id}: pending without owner")
    if args.build_report:
        try:
            report = json.loads(Path(args.build_report).read_text(encoding="utf-8"))
            resolved_host = (report.get("resolved") or {}).get("host") or {}
            if resolved_host.get("dirty"):
                problems.append(f"build report has dirty host files: {resolved_host['dirty']}")
            if report.get("generationId") != candidate.get("generation_id"):
                problems.append("generation_id mismatch vs build report")
            commit_prefix = (resolved_host.get("commit") or "")[:12]
            if commit_prefix and commit_prefix not in pins.get("agent_diva", ""):
                problems.append(f"agent_diva pin does not match report {commit_prefix}")
        except (OSError, json.JSONDecodeError) as exc:
            problems.append(f"cannot read build report: {exc}")
    if args.binary:
        try:
            if digest(Path(args.binary).read_bytes()) != candidate.get("binary_sha256"):
                problems.append("binary sha256 mismatch")
        except OSError as exc:
            problems.append(f"cannot read binary: {exc}")
    for problem in problems:
        print(f"FAIL: {problem}")
    for row_id in failed:
        print(f"FAILED-ROW: {row_id}")
    for row_id in pending:
        print(f"pending: {row_id}")
    print(f"rows: {len(rows)} passed={sum(1 for row in rows if row.get('outcome') == 'passed')} pending={len(pending)} failed={len(failed)}")
    if problems or failed or (pending and args.require_all_passed):
        return 1
    print("OK (historical v1 validation only; not promotable)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", help="acceptance report JSON (v1 historical or v2 strict)")
    parser.add_argument("--build-report", help="legacy v1 build-report.json cross-check")
    parser.add_argument("--binary", help="legacy v1 binary SHA-256 cross-check")
    parser.add_argument("--require-all-passed", action="store_true")
    parser.add_argument("--artifact-root", type=Path, help="root with linux-amd64/ and windows-amd64/ retained candidate files")
    parser.add_argument("--release-source-sha", help="full commit SHA selected by the annotated release tag")
    args = parser.parse_args()
    try:
        doc = json.loads(Path(args.report).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"FAIL: cannot read acceptance report: {exc}")
        return 2
    if not args.artifact_root:
        if doc.get("schema") == "diva.wails-candidate-acceptance/v2":
            print("FAIL: v2 validation requires --artifact-root and --release-source-sha")
            return 2
        return _legacy_validate(args, doc)
    if not args.release_source_sha:
        print("FAIL: strict v2 validation requires --release-source-sha")
        return 2
    artifact_dirs = {platform: args.artifact_root / platform for platform in PLATFORMS}
    errors = validate_candidate(doc, artifact_dirs, args.release_source_sha, args.require_all_passed)
    for error in errors:
        print(f"FAIL: {error}")
    if errors:
        return 1
    print(f"OK: strict v2 candidate accepted for {args.release_source_sha}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
