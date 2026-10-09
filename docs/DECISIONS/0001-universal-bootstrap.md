# ADR-0001: Keep Appbootstrap stack-agnostic

- **Status:** Accepted
- **Date:** 2026-10-09

## Context
One source template should guide creation of fast, stable applications such as media players, GUI tools, web applications, mobile apps and services. A mandatory language/framework would exclude valid use cases.

## Decision
Store canonical cross-cutting engineering rules in `AGENTS.md`; reference them from editor-specific adapters. Provide selectable project profiles, fillable specifications, reusable prompts, a lightweight generator and quality gates. Keep application runtime dependencies out of the template.

## Consequences
- New apps must intentionally select technology, platforms, distribution and measurable budgets.
- The generator emits documentation and agent instructions, not an allegedly functional runtime.
- Platform-specific patterns belong in project architectures or optional profiles, not universal mandates.
- Smaller apps may combine conceptual layers to avoid needless complexity.

