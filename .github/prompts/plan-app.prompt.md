---
description: Turn an app idea into a verifiable, stack-agnostic plan
---
# Plan application

Act as a principal engineer. Follow `AGENTS.md` and `docs/WORKFLOW.md`.

User idea: `${input:idea:Describe the application and users}`

1. Summarize who uses it, primary job, must-haves and explicit non-goals.
2. Identify target OS/devices, constraints, offline/data/security requirements; mark unknowns as **TBD**.
3. Compare at least two plausible stacks if stack is not selected, weighing performance, maintenance, packaging and licenses. No winner by hype.
4. Propose the smallest architecture with module boundaries and state ownership.
5. Draft a first vertical slice, acceptance criteria, failure cases and tests.
6. List measurable startup, responsiveness, memory and reliability budgets, labeling unmeasured targets.
7. Suggest ADRs and a staged implementation checklist.

**Do not implement a full product until scope and architecture have been made explicit.**

