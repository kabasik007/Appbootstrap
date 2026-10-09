# Android engineering rules (extends ../AGENTS.md)

Read this file and ../AGENTS.md before edits. This is a native Android **template**, not a media player.

## Architecture contract
- UI uses Kotlin, single activity, Compose and immutable screen-level UI state.
- Unidirectional data flow: composable event -> ViewModel -> repository; repository Flow -> ViewModel StateFlow -> lifecycle-aware UI.
- :app owns Android entry points, app identity, navigation and composition root.
- :feature:* owns screens and ViewModels, depends on contracts from :core:model, **never** :core:data.
- :core:model owns domain types and repository contracts; no Android framework dependencies.
- :core:data implements repositories; adapters handle storage, network and failure policy.
- Domain/use-case modules are **optional**: introduce when business logic is genuinely reused or hard to test.
- No Hilt, Room, Retrofit, WorkManager or DI framework until a concrete requirement justifies them.
- Keep low-level platform APIs inside adapters and Android entry points, not reusable composables.

## Performance and lifecycle
- Never block the main thread with I/O, decoding, disk indexing or network calls.
- Launch structured coroutines in viewModelScope; collect Flows with collectAsStateWithLifecycle.
- Prefer stable item keys in lists, bounded pagination/caches, cancellation and explicit cleanup.
- Treat background playback/sync as an OS lifecycle concern; only add foreground services,
  Media3 or WorkManager when implementing the corresponding feature.
- Avoid allocations, locks, and logging in real-time audio processing callbacks.
- Measure cold start, memory, jank and battery on a reference device before tuning.

## Security and releases
- No signing keys, API credentials, user data or local.properties in Git.
- Release APKs must be signed with protected Actions secrets; CI debug APK is not a release.
- Validate permissions, exported components, deep links, backups and network security.
- Every new feature needs error/empty/loading states if applicable, tests, and accessibility.

## Verification
- From android/: gradle :core:data:test :app:lintDebug :app:assembleDebug
- Do not claim this passed until CI or an equivalent local environment actually ran it.
- Keep .github/workflows/android-ci.yml and android-release.yml functional.
- Update docs/ARCHITECTURE.md and ADRs for substantial design decisions.
