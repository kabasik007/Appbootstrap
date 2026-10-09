# [PROJECT_NAME] — Architecture

- **Status:** draft
- **Selected profile:** TBD
- **Language/runtime:** TBD (justify)
- **UI/framework:** TBD or not applicable
- **Targets and packaging:** TBD
- **Key architectural decisions:** link to `docs/DECISIONS/`

## Context and constraints
Explain users, scale, deployment, offline requirements and known technical constraints. Link to `docs/PROJECT_BRIEF.md`.

## High-level design
Draw a simple diagram of modules and their dependencies.

## Modules and ownership
| Module | Responsibility | Dependencies | Boundary contract |
| --- | --- | --- | --- |
| TBD | TBD | TBD | TBD |

## State, concurrency and lifecycle
Who owns mutable state? Which operations are cancellable? UI/event-loop/real-time restrictions? Shutdown/restart/recovery?

## Persistence and external integrations
Data schema/versioning, retries/timeouts, transactions, files, networks, permissions, and OS APIs.

## Fault tolerance and observability
Errors, logs, metrics, diagnostics, rate limits, privacy and recovery behavior.

## Security and distribution
Trust boundaries, dependency/license policy, signing, updates and target OS support.

## Quality/performance test strategy
Concrete commands, fixtures, representative hardware, regression gates and links to `docs/PERFORMANCE_BUDGET.md`.

## Trade-offs and open questions
List alternatives, risks and decisions before expanding architecture.

