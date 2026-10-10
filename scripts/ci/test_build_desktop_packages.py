"""The sealed consumer tests real Go packages and skips archived source logs."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "build_desktop", Path(__file__).resolve().parents[1] / "build-desktop.py"
)
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


class TestGoTestPackageTargets(unittest.TestCase):
    def test_excludes_archived_go_test_logs_and_keeps_host_packages(self):
        output = "\n".join(
            [
                "github.com/ProjectViVy/agent-diva/agent-diva-gui",
                "github.com/ProjectViVy/agent-diva/cmd/diva",
                "github.com/ProjectViVy/agent-diva/docs/logs/2026-10-memory-loop-chain/v0.7.0/raw",
                "github.com/ProjectViVy/agent-diva/internal/desktop",
                "github.com/ProjectViVy/agent-diva/internal/speech",
                "github.com/ProjectViVy/agent-diva/tools/wails-probe",
            ]
        )

        self.assertEqual(
            build.go_test_package_targets(output),
            [
                "github.com/ProjectViVy/agent-diva/agent-diva-gui",
                "github.com/ProjectViVy/agent-diva/cmd/diva",
                "github.com/ProjectViVy/agent-diva/internal/desktop",
                "github.com/ProjectViVy/agent-diva/internal/speech",
                "github.com/ProjectViVy/agent-diva/tools/wails-probe",
            ],
        )

    def test_rejects_empty_go_list_output(self):
        with self.assertRaises(SystemExit):
            build.go_test_package_targets("\n")


if __name__ == "__main__":
    unittest.main()
