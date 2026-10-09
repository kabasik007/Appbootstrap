# Desktop native profile

**Best for:** responsive local utilities, music players, graphics tools, low-overhead OS-integrated apps.

## Decide per project
- Target Windows/macOS/Linux versions and architectures; deployment/signing/updater.
- Candidate stacks: Rust + Slint, C++ + Qt, C#/.NET, Swift/AppKit, Kotlin/Compose Desktop. These are **examples**, not required dependencies.
- Native toolkit capabilities, accessibility, drag/drop, fonts, HiDPI, tray, global shortcuts and localization.
- OS/device APIs and isolation: filesystem, audio, GPU, USB, notification, clipboard, power management.

## Architecture hints
- UI owns rendering and presentation state; use asynchronous commands for I/O and long work.
- Isolate device/OS calls behind narrow adapters when cross-platform requirements justify it.
- Define ownership, cancellation and orderly teardown of threads, windows and device sessions.
- For audio players, isolate audio callback constraints from parsing, scanning, metadata, skins and playlists.

## Check before release
Installer/uninstaller, updates, high DPI, keyboard accessibility, suspend/resume, unplugged devices, large datasets, low-RAM and slow disks. Measure cold start and idle resources on target hardware.

