#!/usr/bin/env python3
"""Generate an independent native Android project from the android/ template.

Stdlib only. Refuses to overwrite an existing destination.
"""
from __future__ import annotations

import argparse
import re
import shutil
import tempfile
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "android"
SKIP_PARTS = {".gradle", ".idea", "build", "__pycache__"}
PACKAGE_RE = re.compile(r"[a-z][a-z0-9_]*(?:\.[a-z][a-z0-9_]*)+\Z")


def validate_name(value: str) -> str:
    value = value.strip()
    if not value or len(value) > 80 or any(ord(ch) < 32 for ch in value):
        raise ValueError("Name must be 1-80 printable characters")
    return value


def validate_package(value: str) -> str:
    if not PACKAGE_RE.fullmatch(value):
        raise ValueError("Package must be a lowercase dotted identifier (example: com.example.player)")
    if any(part in {"class", "object", "package", "fun", "val", "var", "in", "is"} for part in value.split(".")):
        raise ValueError("Package cannot contain reserved Kotlin keywords")
    return value


def generate(name: str, package: str, output: Path) -> int:
    name = validate_name(name)
    package = validate_package(package)
    output = output.expanduser().resolve()
    if output.exists():
        raise FileExistsError(f"Destination already exists: {output}")

    output.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".android-bootstrap-", dir=output.parent))
    count = 0
    try:
        safe_gradle_name = re.sub(r"[^A-Za-z0-9_]", "", name) or "AndroidApp"
        path_prefix = Path(*package.split("."))
        for src in SOURCE.rglob("*"):
            if not src.is_file() or SKIP_PARTS.intersection(src.relative_to(SOURCE).parts):
                continue
            rel = src.relative_to(SOURCE)
            components = list(rel.parts)
            for i in range(len(components) - 1):
                if components[i:i + 2] == ["dev", "appbootstrap"]:
                    components[i:i + 2] = list(path_prefix.parts)
                    break
            destination = staging.joinpath(*components)
            destination.parent.mkdir(parents=True, exist_ok=True)

            original = src.read_text(encoding="utf-8")
            updated = original.replace("dev.appbootstrap", package)
            if rel.as_posix() == "app/build.gradle.kts":
                updated = updated.replace(
                    f'applicationId = "{package}.android"',
                    f'applicationId = "{package}"',
                )
            if rel.as_posix() == "settings.gradle.kts":
                updated = updated.replace('rootProject.name = "AppbootstrapAndroid"',
                                          f'rootProject.name = "{safe_gradle_name}"')
            if rel.as_posix() in {
                "app/src/main/res/values/strings.xml",
                "feature/home/src/main/res/values/strings.xml",
                "feature/home/src/main/res/values-uk/strings.xml",
            }:
                updated = updated.replace("Appbootstrap Android", escape(name))
                if rel.as_posix() == "app/src/main/res/values/strings.xml":
                    updated = updated.replace(">Appbootstrap<", f">{escape(name)}<")
            destination.write_text(updated, encoding="utf-8")
            count += 1

        workflows = ROOT / ".github" / "workflows"
        target_workflows = staging / ".github" / "workflows"
        target_workflows.mkdir(parents=True, exist_ok=True)
        for filename in ("android-ci.yml", "android-release.yml"):
            body = (workflows / filename).read_text(encoding="utf-8")
            body = body.replace("gradle --project-dir android ", "gradle ")
            body = body.replace("android/app/build/", "app/build/")
            if filename == "android-ci.yml":
                body = body.replace(
                    '  push:\n    branches: [android]\n    paths:\n      - "android/**"\n      - ".github/workflows/android-ci.yml"\n  pull_request:\n    paths:\n      - "android/**"\n      - ".github/workflows/android-ci.yml"',
                    "  push:\n  pull_request:",
                )
            (target_workflows / filename).write_text(body, encoding="utf-8")
            count += 1

        staging.rename(output)
        return count
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--name", required=True)
    parser.add_argument("--package", required=True, dest="package")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        count = generate(args.name, args.package, args.output)
    except (ValueError, FileExistsError, OSError) as exc:
        parser.error(str(exc))
    print(f"Created '{args.name}' at {args.output} ({count} files).")
    print("Open the output folder in Android Studio. Configure release signing separately.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
