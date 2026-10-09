# Architecture guide (stack-agnostic)

## Choose the simplest sufficient structure
Appbootstrap does not enforce Clean Architecture, hexagonal architecture, DDD, Redux, microservices, Rust or React. Begin with coherent modules. Add layers only when they protect real invariants or reduce coupling.

## Useful conceptual boundaries

| Boundary | Owns | Does not own |
| --- | --- | --- |
| Presentation | input, view state, navigation, rendering | direct DB writes, blocking networking |
| Application | use cases, orchestration, transactions, cancellation | toolkit-specific widget details |
| Domain | rules, invariants, core types | HTTP, SQL, OS APIs where avoidable |
| Infrastructure | DB, HTTP, filesystem, codecs, caches | business policy |
| Platform | OS integration, permissions, installers | application domain rules |

Example dependency flow (not mandatory file hierarchy):

    UI → application → domain
           ↑
       adapters for infrastructure/platform

For very small apps, application and domain may be one module. For complex apps, feature-first organization often beats a giant global "services" folder.

## Contracts
Document input/output, errors, idempotency, timeouts, cancellation, ownership and versioning at each boundary. Keep business decisions in testable units. Parse and validate external data before it enters trusted code.

## Concurrency
- Keep UI/event loop responsive: move blocking work out, schedule bounded jobs, provide progress and cancellation.
- Use message passing or well-defined state ownership; avoid shared mutable state without a clear synchronization story.
- Handle backpressure, partial failures, retry budgets, shutdown and resource cleanup.
- For real-time media, isolate callbacks from filesystem, decoding setup, network waits, heavy allocations and unbounded logging.

## Persistence
Choose storage from workload requirements, not fashion. Include schema/version evolution, backups where relevant, transactional guarantees and corruption recovery. Prefer explicit migrations to silent incompatible changes.

## Observability
Use structured, rate-limited logs with no secrets; define meaningful error categories and optional privacy-respecting diagnostics. Provide reproducible failure reports.

## Dependencies and portability
Keep OS and vendor APIs behind narrow adapters only when genuine portability is required. Inventory version constraints and licensing. Do not assume a cross-platform UI gives cross-platform packaging automatically.

## Project-specific architecture
Generated applications must complete `docs/ARCHITECTURE.md` from `templates/ARCHITECTURE.md` and record consequential decisions as ADRs.

