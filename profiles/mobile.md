# Mobile application profile

**Best for:** Android and/or iOS native or cross-platform apps.

## Decide
- Explicit minimum OS/device versions, store distribution, offline requirements and background execution.
- Candidate stacks: Kotlin, Swift, Flutter, React Native, Kotlin Multiplatform, etc.
- Platform-specific requirements before claiming complete cross-platform parity.

## Design
- Design around lifecycle: foreground/background, suspended processes, rotations, interruptions and system permissions.
- Separate UI, durable local state, network sync and conflict resolution.
- Avoid blocking UI thread; bound battery, network and memory consumption.
- Store secrets in OS key storage as appropriate; ask for minimum permissions.
- Provide accessible controls, readable typography and localization.

## Verify
Cold launch on baseline device, 60/120 Hz smoothness as applicable, battery, network changes, low-memory recovery, offline conflicts, permissions and store packaging/signing.

