# Build, CI and signing

## Development
Requirements: JDK 17, Android SDK Platform 36, Gradle 8.13, Android Studio
compatible with AGP 8.13.2.

```bash
cd android
gradle :core:data:test :app:lintDebug :app:assembleDebug
# Debug artifact: app/build/outputs/apk/debug/app-debug.apk
```

CI uses `gradle/actions/setup-gradle` with Gradle 8.13; a binary Gradle Wrapper
JAR is **not yet checked into this template**. For a standalone copy, run
`gradle wrapper --gradle-version 8.13` and commit generated Gradle Wrapper files
(including gradle-wrapper.jar) after checksum review.

## Signed release (GitHub)
Configure four GitHub Actions repository secrets:
- `ANDROID_KEYSTORE_BASE64`: base64-encoded release keystore file (single line)
- `ANDROID_KEYSTORE_PASSWORD`
- `ANDROID_KEY_ALIAS`
- `ANDROID_KEY_PASSWORD`

**Back up the keystore securely outside the repository.** Do not rotate it
between versions of the same app without a planned Android signing migration.

From the **android branch**, create/push an annotated tag matching
`android-vMAJOR.MINOR.PATCH`, for example `android-v0.1.0`. The
`android-release.yml` workflow validates and builds a signed minified APK,
verifies its signature and publishes a GitHub Release. The app versionCode is
computed from semantic version components (major*1000000+minor*1000+patch).
Keep minor and patch <= 999 and major <= 2000 to avoid overflow.

The Android CI workflow separately uploads a **debug** APK as an Actions artifact.
A debug APK is for testing and is not a signed production release.

## Before public distribution
- Replace package/applicationId `dev.appbootstrap.android`, name and demo UI.
- Add actual icons, privacy policy if needed, localized descriptions and screenshots.
- Review Android target SDK / security/privacy and device testing.
- Check release build shrinking, install, launch, upgrade and rollback on devices.
- Test on real phones including cold start, rotation, low memory and permission denial.
- Confirm APK signing, certificate continuity, semver monotonicity, and backups.
