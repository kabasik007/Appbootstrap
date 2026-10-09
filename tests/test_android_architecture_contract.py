"""Fast static checks of the Android template's declared architectural boundaries."""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class AndroidArchitectureContractTests(unittest.TestCase):
    def test_feature_does_not_depend_on_data_implementation(self):
        build = (ROOT / "android/feature/home/build.gradle.kts").read_text()
        self.assertIn('project(":core:model")', build)
        self.assertNotIn('project(":core:data")', build)

    def test_release_requires_signing_secrets(self):
        flow = (ROOT / ".github/workflows/android-release.yml").read_text()
        self.assertIn("ANDROID_KEYSTORE_BASE64", flow)
        self.assertIn("apksigner", flow)
        self.assertIn("SHA256SUMS", flow)

    def test_ci_runs_real_gradle_checks(self):
        flow = (ROOT / ".github/workflows/android-ci.yml").read_text()
        for task in (":core:data:test", ":app:lintDebug", ":app:assembleDebug"):
            self.assertIn(task, flow)


if __name__ == "__main__":
    unittest.main()
