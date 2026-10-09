# Appbootstrap

> **Branch note — Windows player**: This feature branch contains the **documentation-first Windows Rust music player project** under [apps/windows-media-player/](apps/windows-media-player/README.md), including its [engineering roadmap](apps/windows-media-player/ROADMAP.md). The default `main` branch remains the universal Appbootstrap template.

**Universal, stack-agnostic application engineering bootstrap.** A practical collection of AI coding rules, project specifications, architecture guidance, quality gates, and a small generator. It is **not** a music player, a Rust starter, or a framework.

> Українською: це універсальна основа для розробки стабільних і швидких застосунків разом з AI. Спочатку обираємо профіль і створюємо специфікацію, а вже потім пишемо код.

## What is included

- `AGENTS.md`: canonical instructions for coding agents.
- `.github/instructions/`: focused architecture, security, performance, testing, and quality rules.
- `.github/prompts/`: reusable prompts for planning, features, debugging, optimization, and review.
- `docs/`: conventions, workflow, architecture, security, and performance methodology.
- `templates/`: project brief, architecture, feature specification, decisions, budgets, and release checklist.
- `profiles/`: **desktop-native**, **desktop-webview**, **web**, **mobile**, **backend-cli**.
- `scripts/new_project.py`: generates a clean **documentation + AI-instructions** starter from a selected profile.
- `scripts/validate.py`, `tests/`, and GitHub Actions: checks for the bootstrap itself.

## Quick start

Requires **Python 3.10+**; Python is needed only for the generator and bootstrap checks, *not* for apps generated from it.

```bash
# From the Appbootstrap repository
python3 scripts/validate.py
python3 -m unittest discover -s tests -v

# Create a new project starter OUTSIDE this repository
python3 scripts/new_project.py --name "My App" --profile desktop-native --output ../my-app
```

Generated projects contain rules, prompts, specifications, and the selected profile. **They do not pretend to contain a compiled application or automatically install a runtime.** Choose the implementation stack in the generated `docs/ARCHITECTURE.md`.

## Supported project profiles

| Profile | Typical projects | Optional stacks (examples, not requirements) |
| --- | --- | --- |
| `desktop-native` | Player, local tools, professional GUI | Rust + Slint, C++ + Qt, C#/.NET |
| `desktop-webview` | Cross-platform desktop app with web UI | Tauri, Electron |
| `web` | Websites, dashboards, SaaS | React, Vue, Laravel |
| `mobile` | Android/iOS apps | Kotlin, Swift, Flutter |
| `backend-cli` | Services, workers, command-line tools | Rust, Go, Python, PHP |

## Standard development loop

1. Define scope in `docs/PROJECT_BRIEF.md`, select an architecture, and record decisions.
2. Agree on measurable acceptance criteria and quality/performance budgets.
3. Implement one vertical slice without crossing module boundaries.
4. Verify with formatters, linters, tests, relevant builds, and measurements.
5. Review security, regressions, accessibility (where relevant), and release readiness.

See [Workflow](docs/WORKFLOW.md), [Architecture](docs/ARCHITECTURE.md), [Performance](docs/PERFORMANCE.md), and [Quality gates](docs/QUALITY-GATES.md).

## For AI coding agents

Start with [AGENTS.md](AGENTS.md). Supporting adapters for GitHub Copilot, Claude, and Cursor point to the same canonical rules. Use prompts from `.github/prompts/`. Never claim a build, benchmark, or test succeeded unless it actually ran.

## Design principles

- **Universal before specific:** app-specific code and dependencies belong in generated projects.
- **Simple by default:** boundaries are important, but empty layers and needless abstractions are not.
- **Non-blocking and measurable:** performance claims require target hardware, measurement methods, and baselines.
- **Safe changes:** small diffs, regression tests, and explicit trade-offs.
- **No silent assumptions:** security, licensing, platform support, target stack, and budgets are per-project decisions.

## License

No license has been selected for this repository yet. The owner should choose one before inviting unrestricted third-party reuse.
