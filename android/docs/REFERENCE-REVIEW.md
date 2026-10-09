# Engineering reference review (2026-10-10)

The goal is an excellent **universal Android foundation**, not a giant framework.
These projects were evaluated for reusable design decisions, not copied wholesale.

| Reference | Strong ideas | Template decision |
| --- | --- | --- |
| Google Now in Android (android/nowinandroid) | feature modules, UDF, Flow, lifecycle, convention plugins, benchmark setup | **Adopt** module direction, state/event model; **defer** convention plugins and macrobenchmarks until app grows |
| Google Architecture Templates (android/architecture-templates) | runnable starter, repository, DI, Compose, test coverage | **Adopt** the working vertical slice; leave Hilt/Room optional rather than force them into all apps |
| Google Architecture Samples (android/architecture-samples) | fakeable repositories, unit/integration tests, separate UI | **Adopt** interfaces, tests and explicit data ownership |
| Slack Circuit (slackhq/circuit) | presenter vs UI separation, state/event contracts, Compose-driven navigation | **Adopt principle** of isolated rendering and explicit events; **defer framework** to navigation-heavy apps |
| Cash App Molecule (cashapp/molecule) | build StateFlow streams with Compose runtime, presenter-like logic | **Consider later** for complex derived UI state; StateFlow+ViewModel are sufficient initially |
| Square Workflow (square/workflow-kotlin) | composable state machines for difficult navigation and transitions | **Consider later** for state-heavy transactional flows; don't impose runtime now |
| Square / NIA build-logic | additive Gradle convention plugins to eliminate duplication | **Defer** until multiple modules share enough build config to justify a build-logic included build |

## Selection tests
A tool earns a default dependency only when it:
1. Solves an actual requirement for a generated app.
2. Improves testability, reliability or performance with measurable evidence.
3. Has an ongoing maintenance and upgrade story.
4. Does not force unnecessary runtime/framework coupling.
5. Can be replaced at a narrow architectural seam.

Thus the default app uses Kotlin + Compose + ViewModel + Flow, two pure Kotlin
core modules, one feature module, and a manual composition root. It is
deliberately small, yet can adopt Room, Hilt, Media3, Circuit, Retrofit or
WorkManager when a real application needs them.

## Official references
- https://developer.android.com/topic/architecture
- https://developer.android.com/topic/architecture/recommendations
- https://developer.android.com/topic/modularization
- https://github.com/android/nowinandroid
- https://github.com/android/architecture-templates
- https://github.com/android/architecture-samples
- https://github.com/slackhq/circuit
- https://github.com/cashapp/molecule
- https://github.com/square/workflow-kotlin
- https://github.com/android/nowinandroid/tree/main/build-logic

Architectural influences are attributed here; implementation is original
and no third-party source files or trademarks were imported.
