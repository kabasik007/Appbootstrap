#!/usr/bin/env python3
"""Validate roadmap JSON and generate deterministic human-readable STATUS.md.

This Python helper is for repository maintenance/CI, not a runtime dependency of
the Rust Windows application.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "roadmap" / "roadmap.json"
TARGET = ROOT / "roadmap" / "STATUS.md"
STATUSES = {"todo", "planned", "in_progress", "implemented_unverified", "blocked", "verified"}
PRIORITIES = {"blocker", "high", "medium", "low"}


def validate(data: dict) -> None:
    if data.get("schema_version") != 1:
        raise ValueError("Unsupported roadmap schema")
    phases = data.get("phases")
    if not isinstance(phases, list) or [p.get("id") for p in phases] != [
        f"P{i}" for i in range(8)
    ]:
        raise ValueError("Expected unique, ordered P0 to P7")
    ids = set()
    for phase in phases:
        if phase.get("status") not in STATUSES or not phase.get("gate") or not phase.get("goal"):
            raise ValueError(f"Missing/invalid phase details: {phase.get('id')}")
        if not phase.get("tasks"):
            raise ValueError(f"Empty phase: {phase['id']}")
        for task in phase["tasks"]:
            if task.get("status") not in STATUSES or task.get("priority") not in PRIORITIES:
                raise ValueError(f"Bad task status/priority: {task}")
            if not all(task.get(key) for key in ("id", "title", "verification")):
                raise ValueError(f"Missing task fields: {task}")
            if task["id"] in ids:
                raise ValueError(f"Duplicated task id: {task['id']}")
            ids.add(task["id"])


def render(data: dict) -> str:
    lines = [
        "# ZillaPlayer — development status",
        "",
        "> Generated from [roadmap.json](roadmap.json) by `tools/roadmap_sync.py`.",
        "> Source code present does **not** mean a feature is verified or shipped.",
        "",
        f"**Snapshot:** {data['as_of']}  |  **Phases:** {len(data['phases'])}  |  "
        f"**Tasks:** {sum(len(p['tasks']) for p in data['phases'])}",
        "",
        "## Phase milestones",
        "",
        "| Phase | Focus | Status | Exit gate |",
        "| --- | --- | --- | --- |",
    ]
    for phase in data["phases"]:
        lines.append(
            f"| {phase['id']} | {phase['name']} | {phase['status']} | {phase['gate']} |"
        )
    lines.extend(["", "## Task backlog", ""])
    for phase in data["phases"]:
        lines.extend([
            f"### {phase['id']} — {phase['name']}",
            "",
            f"**Goal:** {phase['goal']}",
            "",
            "| ID | Priority | Task | Status | Acceptance / verification |",
            "| --- | --- | --- | --- | --- |",
        ])
        for task in phase["tasks"]:
            lines.append(
                f"| {task['id']} | {task['priority']} | {task['title']} | "
                f"{task['status']} | {task['verification']} |"
            )
        lines.append("")
    lines.extend([
        "## Quality rule",
        "",
        "A task changes to `verified` only after its acceptance check actually ran ",
        "with evidence (Windows build logs, reproducible test output, or measurements).",
        "Do not fill dates or percentages by guessing; dependencies follow the P0–P7 gates.",
        "",
    ])
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true", help="Verify STATUS.md matches JSON")
    group.add_argument("--write", action="store_true", help="Regenerate STATUS.md")
    args = parser.parse_args()
    data = json.loads(SOURCE.read_text(encoding="utf-8"))
    validate(data)
    expected = render(data)
    if args.write:
        TARGET.write_text(expected, encoding="utf-8")
        print(f"Wrote {TARGET}")
    elif not TARGET.exists() or TARGET.read_text(encoding="utf-8") != expected:
        raise SystemExit("ROADMAP OUT OF SYNC: run python tools/roadmap_sync.py --write")
    else:
        print(f"OK: {len(data['phases'])} phases, "
              f"{sum(len(p['tasks']) for p in data['phases'])} tasks; STATUS.md synced")


if __name__ == "__main__":
    main()
