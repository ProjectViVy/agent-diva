from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


CHECKER = Path(__file__).with_name("check_desktop_boundary.py")
SPEC = importlib.util.spec_from_file_location("check_desktop_boundary", CHECKER)
assert SPEC is not None and SPEC.loader is not None
BOUNDARY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BOUNDARY)


class DesktopBoundaryTests(unittest.TestCase):
    def make_tree(self, root: Path) -> Path:
        (root / "internal/desktop").mkdir(parents=True)
        (root / "cmd").mkdir()
        (root / "agent-diva-gui/src/platform").mkdir(parents=True)
        (root / "agent-diva-gui/src/api").mkdir()
        (root / "internal/desktop/app.go").write_text(
            'package desktop\nimport "github.com/ProjectViVy/agent-vivy/sdk/host/v1"\n', encoding="utf-8"
        )
        (root / "agent-diva-gui/src/platform/desktop-host.ts").write_text(
            "import { Events } from '@wailsio/runtime'\n", encoding="utf-8"
        )
        return root

    def test_sdk_host_import_and_authorized_native_seam_pass(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = self.make_tree(Path(tmp))
            self.assertEqual(BOUNDARY.check_desktop_boundary(root), [])

    def test_rejects_vivy_internal_import(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = self.make_tree(Path(tmp))
            (root / "internal/desktop/escape.go").write_text(
                'package desktop\nimport "github.com/ProjectViVy/agent-vivy/internal/runtime"\n', encoding="utf-8"
            )
            self.assertTrue(any("VIVY internal import" in item for item in BOUNDARY.check_desktop_boundary(root)))

    def test_rejects_direct_native_frontend_call(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = self.make_tree(Path(tmp))
            (root / "agent-diva-gui/src/api/direct.ts").write_text(
                "import { Window } from '@wailsio/runtime'\nWindow.Minimize()\n", encoding="utf-8"
            )
            self.assertTrue(any("direct native" in item for item in BOUNDARY.check_desktop_boundary(root)))

    def test_rejects_direct_native_global_binding(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = self.make_tree(Path(tmp))
            (root / "agent-diva-gui/src/api/direct.ts").write_text(
                "window.go.main.RuntimeService.VivyCall({})\n", encoding="utf-8"
            )
            self.assertTrue(any("direct native" in item for item in BOUNDARY.check_desktop_boundary(root)))

    def test_rejects_browser_provider_fetch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = self.make_tree(Path(tmp))
            (root / "agent-diva-gui/src/api/provider.ts").write_text(
                "fetch('https://api.openai.com/v1/chat/completions')\n", encoding="utf-8"
            )
            self.assertTrue(any("provider fetch" in item for item in BOUNDARY.check_desktop_boundary(root)))

    def test_rejects_packaged_sidecar_or_second_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = self.make_tree(Path(tmp))
            artifact = root / "artifact"
            (artifact / "lib").mkdir(parents=True)
            (artifact / "lib/libvivy-shared.so").write_bytes(b"unexpected sidecar")
            self.assertTrue(any("sidecar/second runtime" in item for item in BOUNDARY.check_desktop_boundary(root, artifact)))


if __name__ == "__main__":
    unittest.main()
