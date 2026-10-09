#!/usr/bin/env python3
"""Create a stack-agnostic project documentation and agent-instruction starter.

Standard library only. Does NOT generate runtime source code or install packages.
"""
from __future__ import annotations

import argparse
import shutil
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PROFILES = (
    "desktop-native",
    "desktop-webview",
    "web",
    "mobile",
    "backend-cli",
)
TEMPLATE_FILES = {
    "PROJECT_BRIEF.md": "PROJECT_BRIEF.md",
    "ARCHITECTURE.md": "ARCHITECTURE.md",
    "PERFORMANCE_BUDGET.md": "PERFORMANCE_BUDGET.md",
    "THREAT_MODEL.md": "THREAT_MODEL.md",
    "RELEASE_CHECKLIST.md": "RELEASE_CHECKLIST.md",
    "QUALITY_GATES.md": "QUALITY-GATES.md",
    "PROJECT_WORKFLOW.md": "WORKFLOW.md",
    "FEATURE_SPEC.md": "FEATURE_SPEC_TEMPLATE.md",
    "ADR.md": "ADR_TEMPLATE.md",
}
COMMON_FILES = (
    "AGENTS.md",
    "CLAUDE.md",
    ".editorconfig",
    ".gitignore",
    ".github/copilot-instructions.md",
    ".cursor/rules/appbootstrap.mdc",
    "docs/PERFORMANCE.md",
    "docs/SECURITY.md",
)


def checked_name(name: str) -> str:
    name = name.strip()
    if not name or any(ord(ch) < 32 for ch in name) or len(name) > 120:
        raise ValueError("Project name must be 1–120 characters without control characters")
    return name


def project_paths(profile: str) -> list[str]:
    files = list(COMMON_FILES)
    files += [
        str(p.relative_to(ROOT))
        for folder in (".github/instructions", ".github/prompts")
        for p in sorted((ROOT / folder).glob("*.md"))
    ]
    files += ["docs/GUIDES/ARCHITECTURE.md", "docs/PROJECT_PROFILE.md",
              "docs/DECISIONS/README.md", "README.md"]
    files += ["docs/" + target for target in TEMPLATE_FILES.values()]
    return sorted(files)


def create_project(name: str, profile: str, output: Path, dry_run: bool = False) -> list[str]:
    name = checked_name(name)
    if profile not in PROFILES:
        raise ValueError(f"Unknown profile: {profile}")
    output = output.expanduser().resolve()
    if output.exists():
        raise FileExistsError(f"Destination already exists: {output}")
    paths = project_paths(profile)
    if dry_run:
        return paths

    output.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=f".{output.name}-", dir=output.parent))
    try:
        def write(relative: str, content: str) -> None:
            target = staging / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(content, encoding="utf-8")

        def copy(relative: str) -> None:
            source = ROOT / relative
            write(relative, source.read_text(encoding="utf-8"))

        for relative in COMMON_FILES:
            copy(relative)
        for folder in (".github/instructions", ".github/prompts"):
            for source in sorted((ROOT / folder).glob("*.md")):
                copy(str(source.relative_to(ROOT)))

        for source_name, target_name in TEMPLATE_FILES.items():
            content = (ROOT / "templates" / source_name).read_text(encoding="utf-8")
            write("docs/" + target_name, content.replace("[PROJECT_NAME]", name))

        write("docs/GUIDES/ARCHITECTURE.md",
              (ROOT / "docs/ARCHITECTURE.md").read_text(encoding="utf-8"))
        write("docs/PROJECT_PROFILE.md",
              (ROOT / "profiles" / f"{profile}.md").read_text(encoding="utf-8"))
        write("docs/DECISIONS/README.md",
              "# Architecture decisions\n\n"
              "Create numbered ADRs here (e.g. 0001-runtime-choice.md). "
              "Start from ../ADR_TEMPLATE.md and capture trade-offs.\n")
        write("README.md",
              f"# {name}\n\nGenerated with [Appbootstrap]"
              "(https://github.com/kabasik007/Appbootstrap).\n\n"
              f"**Profile:** {profile}\n\n"
              "This repository starts with **planning documents and AI instructions**, "
              "not a compiled executable or preselected framework.\n\n"
              "1. Fill [Project brief](docs/PROJECT_BRIEF.md).\n"
              "2. Choose stack and fill [Architecture](docs/ARCHITECTURE.md).\n"
              "3. Set [Performance budget](docs/PERFORMANCE_BUDGET.md), "
              "[Threat model](docs/THREAT_MODEL.md), and "
              "[Quality gates](docs/QUALITY-GATES.md).\n"
              "4. Follow [Workflow](docs/WORKFLOW.md), build one vertical slice, "
              "then set up stack-specific CI.\n\n"
              "AI agents should start with [AGENTS.md](AGENTS.md).\n")
        staging.rename(output)
        return paths
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--name", required=True, help="Human-readable project name")
    parser.add_argument("--profile", required=True, choices=PROFILES)
    parser.add_argument("--output", required=True, type=Path, help="New destination directory")
    parser.add_argument("--dry-run", action="store_true", help="List generated paths only")
    args = parser.parse_args()
    try:
        paths = create_project(args.name, args.profile, args.output, args.dry_run)
    except (ValueError, FileExistsError, OSError) as exc:
        parser.error(str(exc))
    if args.dry_run:
        print("\n".join(paths))
    else:
        print(f"Created {args.name!r} ({args.profile}) at {args.output}")
        print(f"{len(paths)} starter files. Choose a tech stack in docs/ARCHITECTURE.md.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

