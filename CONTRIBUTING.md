# Contributing to Appbootstrap

Keep this repository reusable across **desktop, web, mobile, and backend/CLI** rather than customizing it for one app.

1. Open/describe the need and target user of a new rule or profile.
2. Prefer editing canonical `AGENTS.md` / `docs/` over duplicating long rules in editor adapters.
3. Add templates or prompts only if they have a distinct repeatable job and clear outputs.
4. Keep Python generator code dependency-free; test changes with `python3 -m unittest discover -s tests -v`.
5. Run `python3 scripts/validate.py` and verify relative documentation links.
6. Do not add mandatory app runtime dependencies or theoretical performance guarantees.

Use small pull requests, explain compatibility and provide command output for executed checks. License selection remains with the repository owner.

