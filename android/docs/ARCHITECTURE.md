# Android architecture — decision record and extension map

## Why this structure
A compact, feature-first implementation of Android's recommended layered architecture
with UDF and testable repository boundaries. It is intentionally **not** a 30-module
clone of Now in Android. Scale only when concrete product requirements demand it.

```text
:app (entry point, composition root, navigation)
  +--> :feature:home (Compose -> ViewModel -> UI State)
  |           |
  |           +--> :core:model (model + repository interface)
  |
  +--> :core:data (in-memory example adapter)
              |
              +--> :core:model

No arrow from :feature:home to :core:data.
```

## Invariants
1. ViewModel emits an immutable StateFlow; composables render it and send events.
2. Repository interface lives in :core:model. UI does not use DB, HTTP, OS APIs directly.
3. :app wires interfaces to implementations; no global mutable singleton service locator.
4. Repository changes should not force screen/API changes.
5. In-memory sample data **does not survive process death**. Persistence is not claimed.
6. Errors, loading, retries and offline behavior are designed when introducing I/O.
7. Avoid one-use use cases, adapters, DI containers or layers without a clear boundary.

## Growth path
- **Another screen:** add :feature:settings (or similar) and wire it from :app;
  introduce Navigation Compose once the second destination exists.
- **Persistent local data:** implement ItemRepository using Room in :core:data,
  provide an explicit migration strategy and adapter tests.
- **Networking:** add a remote data source inside :core:data, with timeouts and
  a single source of truth; define offline and retry policy first.
- **Complex business rules:** add :core:domain with use cases, depending on repository
  contracts only, not Android or concrete adapters.
- **Dependency injection:** introduce Hilt only when wiring becomes error-prone
  or the dependency graph becomes large.
- **Audio/video apps:** isolate Media3 player/service adapters and lifecycle from Compose;
  never decode or touch disk from real-time audio callbacks.
- **Design system:** extract :core:designsystem when multiple features actually reuse components.
- **Performance:** add Baseline Profiles and Macrobenchmark after target use cases and
  real target devices are defined; don't fabricate improvements.

## Target and toolchain
- minSdk 26; compileSdk / targetSdk 36 (update intentionally per product).
- Java 17; Gradle 8.13; AGP 8.13.2; Kotlin 2.2.21; Jetpack Compose BOM 2026.02.00.
- Version catalog: gradle/libs.versions.toml.
- Build / dependency versions deliberately pinned for predictability.

## References and licenses
- Android architecture: https://developer.android.com/topic/architecture
- Recommendations: https://developer.android.com/topic/architecture/recommendations
- Modularization trade-offs: https://developer.android.com/topic/modularization
- Now in Android: https://github.com/android/nowinandroid
- Architecture samples: https://github.com/android/architecture-samples
- Architecture starter templates: https://github.com/android/architecture-templates

Architecture concepts are inspired by publicly documented patterns. No source
files were copied from the referenced projects. Check dependency/project licenses
before redistributing a branded application.
