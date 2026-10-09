# Android quality gates

CI automatically runs JVM unit tests, Android Lint and assembles an Android debug APK.
This is a **minimum safety net**, not proof of performance, device compatibility or
production readiness.

For each feature:
1. Keep ViewModel inputs/events explicit, screen state immutable, and repositories fakeable.
2. Write unit tests for normal, invalid, empty and concurrent paths when relevant.
3. Implement UI/instrumentation tests for critical flows on an emulator or device.
4. Check lifecycle, rotation, process recreation, accessibility and localized strings.
5. Validate permission scope, log privacy and dependency updates.
6. Record measured startup/frame/battery/memory budgets on agreed target phones.

For production release:
- Verify signed release **APK** installs, launches, updates and passes representative
  UI/functional tests on Android API 26, current API and a low-memory device.
- Validate real persistent storage migrations, offline sync and networking if added.
- Fail the release if signing secrets are missing; never publish an unsigned APK.
- Only enable tracking/analytics and crash reporting with explicit privacy review.
- Do not claim performance or passing tests unless measured/executed.
