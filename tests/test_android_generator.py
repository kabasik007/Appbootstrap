"""Lightweight regression checks for the independent Android starter generator."""
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
from new_android_project import generate, validate_package  # noqa: E402


class AndroidProjectGeneratorTests(unittest.TestCase):
    def test_package_validation(self):
        self.assertEqual("com.example.player", validate_package("com.example.player"))
        with self.assertRaises(ValueError):
            validate_package("COM.Bad-Name")
        with self.assertRaises(ValueError):
            validate_package("com.example.class")

    def test_generates_independent_project_and_namespaces(self):
        with tempfile.TemporaryDirectory() as tmp:
            dest = Path(tmp) / "player"
            count = generate("My Player", "com.example.player", dest)
            self.assertGreater(count, 10)
            app = (dest / "app" / "build.gradle.kts").read_text()
            self.assertIn('applicationId = "com.example.player"', app)
            self.assertIn('namespace = "com.example.player.android"', app)
            self.assertTrue((dest / "app" / "src" / "main" / "java" / "com" /
                             "example" / "player" / "android" / "MainActivity.kt").exists())
            self.assertIn("gradle :core:data:test",
                          (dest / ".github" / "workflows" / "android-ci.yml").read_text())
            self.assertNotIn("android/app/build/",
                             (dest / ".github" / "workflows" / "android-release.yml").read_text())
            with self.assertRaises(FileExistsError):
                generate("My Player", "com.example.player", dest)


if __name__ == "__main__":
    unittest.main()
