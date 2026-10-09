from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "build-desktop.py"
SPEC = importlib.util.spec_from_file_location("build_desktop", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
BUILD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)


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
                host_dir=str(host), vivy_dir=str(vivy), laputa_dir=str(laputa), output=str(root / "out")
            )
            with (
                patch.object(BUILD, "stage_tracked", side_effect=stage_tracked),
                patch.object(BUILD, "build_lock", return_value={}),
                patch.object(BUILD, "sdk_pack", side_effect=sdk_pack),
                patch.object(BUILD, "run"),
            ):
                BUILD.mode_test(args)


if __name__ == "__main__":
    unittest.main()
