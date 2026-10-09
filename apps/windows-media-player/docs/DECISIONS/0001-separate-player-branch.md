# ADR-0001 — Separate media player feature branch

- Status: Accepted for planning
- Date: 2026-10-10

## Context
Appbootstrap on `main` is intended as a reusable technology-neutral engineering template. A Windows audio player has highly specialized architecture and external media/tool dependencies.

## Decision
Use a dedicated branch `apps/windows-media-player` and a namespaced app folder `apps/windows-media-player/`. Retain the universal bootstrap on `main` without product-specific changes. Keep first commit documentation-only until sound playback technology is validated.

## Consequences
- No product code is blended into the universal template.
- This branch will diverge from `main`; if the product expands, create a separate repository from the template and migrate player-specific commits there.
- The app name (ZillaPlayer) and exact UI/audio dependency versions are provisional.
- Implementation begins only after P0 feasibility tests with real build and measurable baseline.

