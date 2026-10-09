---
description: Refactor while retaining existing behavior and public contracts
---
# Safe refactor

Follow `AGENTS.md` and `docs/ARCHITECTURE.md`. Refactor target: `\${input:target:Module or code smell}`

Inspect tests, dependencies, consumers, file formats, schema and public APIs first. Explain invariant behavior and bounded scope. Characterize existing behavior with tests; make incremental changes; run all relevant checks; compare performance where affected. Document breaking changes for explicit approval. Avoid rewriting unrelated modules.

