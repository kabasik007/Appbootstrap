#!/usr/bin/env python3
"""Validate Appbootstrap repository structure and local Markdown links."""
from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent
REQUIRED = (
    "README.md", "AGENTS.md", "CLAUDE.md", ".editorconfig",
    ".github/copilot-instructions.md", ".github/workflows/validate.yml",
    "docs/WORKFLOW.md", "docs/ARCHITECTURE.md", "docs/PERFORMANCE.md",
    "docs/SECURITY.md", "docs/QUALITY-GATES.md",
    "scripts/new_project.py", "scripts/validate.py",
    "templates/PROJECT_BRIEF.md", "templates/ARCHITECTURE.md",
    "templates/QUALITY_GATES.md", "templates/PROJECT_WORKFLOW.md",
)
LINK = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")


def validate(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    for name in REQUIRED:
        if not (root / name).is_file():
            failures.append(f"Missing required file: {name}")
    for md in sorted(root.rglob("*.md")):
        if ".git" in md.relative_to(root).parts:
            continue
        in_fence = False
        for number, line in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.lstrip()
            if stripped.startswith("```") or stripped.startswith("~~~"):
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            for match in LINK.finditer(line):
                dest = match.group(1).strip().split()[0].strip("<>")
                if not dest or dest.startswith(("#", "https://", "http://", "mailto:", "data:")):
                    continue
                relative = unquote(dest.split("#", 1)[0].split("?", 1)[0])
                if not relative:
                    continue
                if not (md.parent / relative).exists():
                    failures.append(
                        f"{md.relative_to(root)}:{number}: broken local link: {dest}"
                    )
    return failures


def main() -> int:
    failures = validate()
    if failures:
        for item in failures:
            print("ERROR:", item, file=sys.stderr)
        return 1
    print("OK: required files and local Markdown links")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

