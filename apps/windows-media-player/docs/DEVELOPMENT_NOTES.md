# Local Windows build and test notes

## Prerequisites
- Windows 10/11 x64.
- Latest compatible **Rust stable MSVC** toolchain (`rustup toolchain install stable`).
- Visual Studio 2022 Build Tools, C++ toolset and Windows SDK.
- Internet access on first build to download Rust dependencies.
- A functioning Windows audio output device for manual audio tests.

## Build
```powershell
git clone -b apps/windows-media-player https://github.com/kabasik007/Appbootstrap.git
cd Appbootstrap\apps\windows-media-player
cargo check
cargo test
cargo run
cargo build --release
```

## Manual smoke test (requires hearing output)
1. Launch UI. All pages should navigate without exceptions.
2. Select **Open File**, choose a small legal MP3; track title and audio status should update; playback begins.
3. Pause and resume. Use **Stop**, then **Play** to replay the selected file.
4. Change volume; output should respond.
5. Drag seek slider to 50%; status should jump if source supports seeking.
6. Repeat with FLAC and WAV.
7. Open a corrupt/missing source after a valid one: old track must continue, show a recoverable error.
8. Disconnect/default-switch device and record behavior; automatic reconnect is not implemented.
9. Open Downloads/Tasks/Plans: clearly labeled demos, no downloads performed.
10. Check Task Manager CPU/RAM and taskbar UI responsiveness under 30-minute playback.

## Failure reports
Attach Windows version, CPU/RAM, device model, exact cargo command output, reproducible steps, and a legal non-sensitive test media file or metadata. Never upload private audio, tokens or device keys.

## Limits
No `Cargo.lock` was generated in the assistant environment. Commit the generated lockfile after a successful real Cargo resolution; pin Slint/Rodio/rfd direct dependencies until CI validation. No test or performance measurement should be claimed passed solely because this file exists.
