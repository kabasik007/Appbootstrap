# Engineering workflow

## 0 — Bootstrap
Run `python3 scripts/new_project.py --name "My App" --profile web --output ../my-app` from the source repository. In the generated project, complete `docs/PROJECT_BRIEF.md`, `docs/ARCHITECTURE.md`, `docs/PERFORMANCE_BUDGET.md` and `docs/THREAT_MODEL.md`. Select licenses and actual build tools separately.

## 1 — Discovery and scope
Document users, platforms, non-goals, failure modes, integrations, data types, privacy and deployment. Inventory existing systems and clarify unknown constraints. Do not design for imagined future products.

## 2 — Architecture decision
Draw the boundaries, show dependency direction and choose technology using required capabilities, team skills, support, ecosystem and measurable constraints. Record significant alternatives in `docs/DECISIONS/`.

## 3 — Slice planning
Write a feature specification with acceptance criteria, error cases and tests. Prefer a runnable end-to-end slice over five unconnected abstractions. Include rollback and migration needs.

## 4 — Build and verify
Implement small commits; automate formatting, static checks, tests and packaging appropriate to the chosen stack. Treat cancelled tasks, resource cleanup, permission errors and corrupt inputs as first-class behaviors.

## 5 — Performance gate
Baseline representative hardware and data. Measure cold/warm startup, interaction p95, memory, sustained work, CPU and I/O where relevant. Diagnose regressions with profilers and traces, not guesses.

## 6 — Security and release
Review secrets, permissions, data retention, logging, update/distribution and third-party licensing. Complete `docs/RELEASE_CHECKLIST.md`; report unrun checks as **not verified**, never as green.

## AI operating loop
**Inspect → Specify → Plan → Implement → Verify → Review → Document**. Each iteration should end with a factual summary of files changed and checks actually run.

