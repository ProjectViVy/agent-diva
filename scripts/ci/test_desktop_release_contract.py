from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = (ROOT / ".github/workflows/desktop-release.yml").read_text(encoding="utf-8")


class DesktopReleaseContractTests(unittest.TestCase):
    def test_publication_depends_on_aggregate_acceptance_and_downloads_same_bytes(self) -> None:
        self.assertIn("candidate_run_id", WORKFLOW)
        self.assertIn("acceptance_ref", WORKFLOW)
        self.assertIn("--require-all-passed", WORKFLOW)
        self.assertIn("--artifact-root", WORKFLOW)
        self.assertIn("download-artifact@v4", WORKFLOW)
        candidate = re.search(r"(?ms)^  candidate-build:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|\Z)", WORKFLOW)
        accept = re.search(r"(?ms)^  all-platform-acceptance:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|\Z)", WORKFLOW)
        publish = re.search(r"(?ms)^  publish:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|\Z)", WORKFLOW)
        self.assertIsNotNone(candidate)
        self.assertIsNotNone(accept)
        self.assertIsNotNone(publish)
        self.assertIn("contents: read", candidate.group(1))
        self.assertIn("linux-amd64", candidate.group(1))
        self.assertIn("windows-amd64", candidate.group(1))
        self.assertIn("needs: [candidate-build]", accept.group(1))
        self.assertNotIn("gh release upload", candidate.group(1))
        self.assertIn("needs: [all-platform-acceptance]", publish.group(1))
        self.assertIn("gh release upload", publish.group(1))
        self.assertIn("candidate_run_id", publish.group(1))
        self.assertNotIn("--clobber", publish.group(1))
        for forbidden in ("scripts/build-desktop.py", "sdk pack", "go install", "signtool", "codesign"):
            self.assertNotIn(forbidden, publish.group(1))
        self.assertIn("refs/tags/", accept.group(1))
        self.assertIn("rev-parse", accept.group(1))

    def test_missing_platform_leg_cannot_reach_public_upload(self) -> None:
        accept = re.search(r"(?ms)^  all-platform-acceptance:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|\Z)", WORKFLOW)
        publish = re.search(r"(?ms)^  publish:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|\Z)", WORKFLOW)
        self.assertIsNotNone(accept)
        self.assertIsNotNone(publish)
        self.assertIn("set -e", accept.group(1))
        self.assertIn("check_wails_candidate.py", accept.group(1))
        self.assertIn("needs: [all-platform-acceptance]", publish.group(1))
        self.assertNotIn("if: always()", publish.group(1))

    def test_annotated_tag_peel_is_the_candidate_source_identity(self) -> None:
        accept = re.search(r"(?ms)^  all-platform-acceptance:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|\Z)", WORKFLOW)
        publish = re.search(r"(?ms)^  publish:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|\Z)", WORKFLOW)
        self.assertIsNotNone(accept)
        self.assertIsNotNone(publish)
        for job in (accept.group(1), publish.group(1)):
            self.assertIn('cat-file -t "refs/tags/$RELEASE_TAG"', job)
            self.assertIn('rev-parse "refs/tags/$RELEASE_TAG^{commit}"', job)
            self.assertIn('test "$peeled" = "$checked_out"', job)
            self.assertIn("--release-source-sha \"$peeled\"", job)
            self.assertNotIn("fetch --force", job)
            self.assertNotIn("git tag ", job)
            self.assertNotIn("git push ", job)

    def test_archive_tags_and_pinned_w6_approval_gate_upload(self) -> None:
        accept = re.search(r"(?ms)^  all-platform-acceptance:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|\Z)", WORKFLOW)
        publish = re.search(r"(?ms)^  publish:\n(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|\Z)", WORKFLOW)
        self.assertIsNotNone(accept)
        self.assertIsNotNone(publish)
        for job in (accept.group(1), publish.group(1)):
            self.assertIn('ARCHIVE_TAG="archive/tauri-cabi-20261004"', job)
            self.assertIn('cat-file -t "refs/tags/$ARCHIVE_TAG")" = tag', job)
            self.assertIn("5444795a2d9db31e158c2cf009d64697e6289e50", job)
            self.assertIn("fc559e6b03ce4e65c0099b9745855dccc4fb067e", job)
        self.assertIn("acceptance_sha", accept.group(1))
        self.assertIn("needs.all-platform-acceptance.outputs.acceptance_sha", publish.group(1))
        self.assertIn("Full DIVA commit SHA", WORKFLOW)


if __name__ == "__main__":
    unittest.main()
