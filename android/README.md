# Appbootstrap Android — native starter

**A reusable, deliberately lean Android architecture**, not a finished media player.
This directory is an independent Gradle project inside the Appbootstrap `android` branch.

## Start
- JDK 17, Android SDK Platform 36, Gradle 8.13.
- Open `android/` as a project in Android Studio, or run:

```bash
cd android
gradle :core:data:test :app:lintDebug :app:assembleDebug
```

The example app lets you enter items and observe a reactive list. It demonstrates
a complete one-way UI -> ViewModel -> repository -> Flow -> UI architecture.
Demo items are **in memory only**, not persisted across process termination.

## Modules
| Module | Purpose |
| --- | --- |
| `:app` | Android lifecycle, single activity, app-wide composition root |
| `:feature:home` | Compose UI, ViewModel and immutable state |
| `:core:model` | Pure Kotlin models and repository contracts |
| `:core:data` | Thread-safe in-memory demo repository; future adapters |

No Hilt, Room, networking, ads, analytics, sensitive permissions or background
services are installed by default. Add only those your actual application needs.

## Make a new app
From the **repository root**, use:

```bash
python3 scripts/new_android_project.py \
  --name "My Player" \
  --package com.example.myplayer \
  --output ../my-player
```

The generator creates an independent project with its own GitHub workflows.
Its `--package` value is the **final application ID**; the app code namespace
will be `<package>.android` and sibling modules `<package>.core.*`.

## Automation
- `.github/workflows/android-ci.yml` runs checks and uploads a **debug** APK.
- `.github/workflows/android-release.yml` publishes a **signed** APK for
  `android-v1.0.0`-style tags only after signing secrets are configured.
- A local Gradle Wrapper JAR is not bundled. CI installs the pinned version
  with the official setup-gradle action. See [release guide](docs/RELEASE.md).

## Engineering contract
Start with [AGENTS.md](AGENTS.md) and
[architecture](docs/ARCHITECTURE.md). Respect dependency direction and add
only evidence-backed complexity. A multimedia app can add Media3 and a playback
service later without coupling decoding to the UI.

This template is original implementation based on common Android architecture
patterns, not a copied third-party codebase.
