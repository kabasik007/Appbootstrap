"""Minimal tests for repository contract/link checks."""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from validate import validate  # noqa: E402


class ValidateTests(unittest.TestCase):
    def test_repo_contract(self) -> None:
        self.assertEqual(validate(), [])

    def test_missing_files_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            self.assertTrue(any("Missing required file" in item
                                for item in validate(Path(temp))))


if __name__ == "__main__":
    unittest.main()

