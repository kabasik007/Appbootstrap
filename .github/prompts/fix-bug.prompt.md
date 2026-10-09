---
description: Debug a reproducible defect without speculative large rewrites
---
# Fix bug

Follow `AGENTS.md`. Problem: `\${input:bug:Observed failure and reproduction steps}`

1. Reproduce the observed behavior, or state precisely why reproduction is unavailable.
2. Trace entry point and actual data flow; distinguish symptoms from root cause.
3. Add a failing test/reproduction script where practical.
4. Fix the smallest cause, not adjacent unrelated architecture.
5. Run regression tests and sanity checks; compare before/after behavior.
6. Report root cause (evidence-based), changed files, tests run and unresolved hypotheses.

