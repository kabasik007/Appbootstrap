# AGENTS.md — Appbootstrap AI engineering contract

This file is the single source of truth for coding agents. Read it BEFORE changing a generated project. Rules apply to every stack unless a project-specific, documented constraint supersedes them.

## Mission
Build **correct, maintainable, secure, responsive, measurable** applications. Avoid imaginary results, unnecessary dependencies, and speculative rewrites. This repository is a **universal starter**, not a mandate to use a particular language or framework.

## First inspection (mandatory)
1. Read `docs/PROJECT_BRIEF.md`, `docs/ARCHITECTURE.md`, `docs/PROJECT_PROFILE.md`, `docs/PERFORMANCE_BUDGET.md`, relevant decisions and existing code. In this template repository, consult `templates/` and `profiles/` instead.
2. Inventory entry points, integrations, persistence, target platforms, build/test commands and existing tests.
3. Identify unknowns explicitly. Do not invent API behavior, package availability, benchmark outcomes, or undocumented requirements.

## Change workflow
1. Define intended behavior, acceptance criteria, non-goals and affected modules.
2. Produce a short plan for nontrivial changes; state risks and how to verify.
3. Prefer small, reversible vertical slices; preserve public contracts and existing functionality unless change is requested.
4. Keep dependency direction clear and business logic independent of UI/infrastructure where that separation adds value.
5. Add/update tests covering success, failure, edge cases and regressions.
6. Run available format, lint, typecheck, tests and build. Report exact commands, results, and gaps.
7. Update specs, ADRs and performance baselines when behavior or architecture changes.

## Non-negotiable rules
- Never invent test results, measurements, files, commits, supported platforms, or third-party APIs.
- Never claim "fixed" merely because code was edited; confirm with a reproducible check.
- No blocking disk/network/heavy computation on latency-sensitive UI or real-time threads.
- Never use secrets in source, prompts, screenshots, fixtures, commits or logs. Validate untrusted input at boundaries.
- Default to bounded queues/caches, cancellation, resource cleanup, timeout handling, graceful failures and informative diagnostics.
- No architecture astronautics: for simple projects, small modules are better than empty layers or interfaces with one accidental implementation.
- Avoid broad refactoring when fixing a local defect; do not delete files, change licenses, migrate schemas or break compatibility silently.
- Do not use force pushes or destructive commands without explicit authorization.
- Dependencies must have a clear need, acceptable licenses, active support and a strategy for updates.
- The selected profile is guidance, not a hard-coded stack. Confirm target OS, hardware, offline requirements and distribution model.

## Architecture boundaries (adapt to project size)
- **Presentation**: UI, input handling, state projection; no hidden file/network/database work on UI thread.
- **Application**: use cases, orchestration, cancellation, state transitions.
- **Domain**: core concepts, invariants and behavior; framework-independent when practical.
- **Infrastructure/platform**: persistence, network, system APIs, audio, file I/O, adapters.
- Dependencies usually flow inward; use narrow interfaces for external effects. Do not split files merely to follow labels.

## Performance and reliability
- Define target hardware + workload + warm/cold conditions, then baseline before optimizing.
- Record p50/p95/p99 latency where appropriate, startup time, memory, CPU, I/O, frame timing and failure modes.
- Separate background work from latency-sensitive paths; avoid unbounded concurrency and memory growth.
- For audio/video/real-time callbacks: no blocking locks, allocations or file I/O in critical callbacks unless the specific API safely permits them.
- Optimize measured bottlenecks, not intuition. Keep performance tests comparable and repeatable.
- If budget unknown, label it **TBD**; never convert example numbers into product promises.

## Output expectations for agents
For each meaningful change, provide:
- **Scope**: what changed and why
- **Files**: touched paths
- **Verification**: commands run and actual outcomes
- **Risks/next steps**: anything unresolved

## Instruction precedence
User requirements and project-specific design decisions take precedence over generic examples, except security and correctness constraints. Keep this file canonical: editor-specific adapters should reference it, not duplicate drifting rules.

Read `docs/WORKFLOW.md`, `docs/ARCHITECTURE.md`, `docs/PERFORMANCE.md`, `docs/SECURITY.md` and `docs/QUALITY-GATES.md` for details.

