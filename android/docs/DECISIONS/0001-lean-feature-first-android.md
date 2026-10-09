# ADR-0001 — Lean feature-first native Android starter

Status: Accepted
Date: 2026-10-10

## Problem
We need one reliable starting point for different kinds of Android apps:
utilities, media players, storefront companions, and background-enabled tools.
A fully fledged enterprise app skeleton introduces dependencies and build-time
overhead before any real features exist; a single massive app module loses boundaries.

## Decision
- Kotlin + Jetpack Compose + Material 3 as the default native Android UI.
- One entry activity and a feature-first module layout.
- Use StateFlow, immutable screen state, and explicit actions, collected with lifecycle awareness.
- Keep pure Kotlin domain/repository contracts separate from Android framework code.
- Wire a default concrete repository through a manual AppContainer in :app.
- Provide a testable in-memory vertical slice plus Android lint, unit-test and APK CI.
- Use reproducible pinned tool versions, and optional signed GitHub Releases.

## Trade-offs
- No persistent data in starter: users must implement and test Room/DataStore when needed.
- No bundled Gradle Wrapper JAR yet: pinned Gradle is installed in CI; local wrapper
  generation remains an explicit task.
- No Hilt/DI graph, Retrofit, navigation framework, baseline profile or media service
  until a real project requires it.
- Four modules are a deliberate small compromise between isolation and build complexity.

## Revisit when
- Three or more features need repeated Gradle config: add convention plugins.
- Complex DI wiring grows: evaluate Hilt.
- Navigation/state restoration complexity emerges: evaluate type-safe Navigation/Circuit.
- App startup or jank measurements reveal issues: add benchmark and baseline-profile modules.
- Audio playback is required: add Media3 + MediaSessionService independently of UI.

See ../REFERENCE-REVIEW.md for upstream comparison and attribution.
