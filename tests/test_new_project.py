"""Black-box tests for the dependency-free project generator."""
from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "new_project.py"


class NewProjectTests(unittest.TestCase):
    def run_cli(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), *args],
            text=True, capture_output=True, check=False,
        )

    def test_each_profile_generates_starter(self) -> None:
        for profile in ("desktop-native", "desktop-webview", "web",
                        "mobile", "backend-cli"):
            with self.subTest(profile=profile), tempfile.TemporaryDirectory() as temp:
                output = Path(temp) / "new-app"
                result = self.run_cli("--name", "My App", "--profile", profile,
                                      "--output", str(output))
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue((output / "AGENTS.md").is_file())
                self.assertTrue((output / ".github/prompts/plan-app.prompt.md").is_file())
                self.assertTrue((output / "docs/PROJECT_PROFILE.md").is_file())
                self.assertTrue((output / "docs/ARCHITECTURE.md").is_file())
                self.assertTrue((output / "docs/QUALITY-GATES.md").is_file())
                self.assertIn("My App", (output / "docs/PROJECT_BRIEF.md").read_text())
                self.assertNotIn("[PROJECT_NAME]",
                                 (output / "docs/PROJECT_BRIEF.md").read_text())
                self.assertNotIn("Not configured",
                                 (output / "docs/PROJECT_PROFILE.md").read_text())
                self.assertFalse((output / "src").exists(),
                                 "Generator must not pretend to provide an app runtime")

    def test_refuse_to_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "app"
            output.mkdir()
            (output / "sentinel").write_text("keep")
            result = self.run_cli("--name", "App", "--profile", "web",
                                  "--output", str(output))
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual((output / "sentinel").read_text(), "keep")

    def test_dry_run_does_not_create(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "app"
            result = self.run_cli("--name", "App", "--profile", "backend-cli",
                                  "--output", str(output), "--dry-run")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("docs/PROJECT_PROFILE.md", result.stdout)
            self.assertFalse(output.exists())

    def test_reject_invalid_name(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            result = self.run_cli("--name", " ", "--profile", "web",
                                  "--output", str(Path(temp) / "app"))
            self.assertNotEqual(result.returncode, 0)

    def test_reject_invalid_profile(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            result = self.run_cli("--name", "App", "--profile", "unknown",
                                  "--output", str(Path(temp) / "app"))
            self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()

