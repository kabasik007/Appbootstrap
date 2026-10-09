# Implementation status — ZillaPlayer

## Implemented in this branch

- Native Rust/Slint application scaffold (Windows-first).
- Four navigable **UI prototypes**: Player, Downloads, Tasks, Plans.
- Equalizer controls and download entry visual widgets; no underlying effects.
- GitHub Actions Windows check workflow.

## Not implemented — do not describe as working

- Actual audio decoding, WASAPI output, play/pause/seek, DSP, FFT, media library.
- yt-dlp/FFmpeg integration, files/downloads, persistent queue or progress.
- Real task management, roadmap editing, user accounts or cloud syncing.
- Packaging, executable signing, measurements or a published Windows installer.

## Run locally (requires Windows + Rust MSVC toolchain)

Install Visual Studio Build Tools with Desktop development with C++ plus rustup stable.
Run:

```powershell
cd apps/windows-media-player
cargo run
```

For build checks:

```powershell
cargo check
cargo test
cargo build --release
```

The Windows CI runner must validate that the Slint syntax and toolchain are compatible; development has not been verified by a local Windows build in this environment. Version in Cargo.toml is a temporary pinned baseline; commit Cargo.lock once verified.

## Next engineering step

Phase P0: validate Slint compile and Windows rendering, then implement Symphonia → PCM ring → cpal/WASAPI audio output vertical slice in independent crates. Keep UI actions labelled as demo-only until actually connected to sound.
