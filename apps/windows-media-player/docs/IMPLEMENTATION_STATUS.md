# ZillaPlayer — implementation status

## Phase
**P1 / local playback alpha — code committed, Windows build pending verification.**

### Implemented in source
- Native Rust + Slint UI with Player / Downloads / Tasks / Plans pages and a local file picker.
- Background audio-control worker with bounded non-blocking UI status channel.
- Local MP3, FLAC, WAV, OGG/M4A decoding supported through **Rodio 0.21.1 → Symphonia → CPAL/Windows WASAPI** (subject to build and file-format validation).
- Play / Pause / Stop / Restart / percentage Seek / output volume.
- Transport state, progress/volume clamping and time formatting regression tests.
- Invalid replacement file retains playback of existing track.
- CI configuration for Windows check / Rust tests / release build.

### Deliberately not implemented
- A real equalizer, audio effects, ReplayGain, meters, live FFT and GPU spectrum.
- Library indexing, queue/playlist playback, next/previous track and persistence.
- yt-dlp/FFmpeg jobs, conversion, plugin loading, update system.
- Working task-management data or roadmap syncing.
- Windows EXE build confirmation, installer, benchmarks and release signing.

### Run the alpha locally on Windows
Install Rust stable (MSVC toolchain) and Visual Studio Build Tools (Desktop development with C++).

```powershell
git clone -b apps/windows-media-player https://github.com/kabasik007/Appbootstrap.git
cd Appbootstrap/apps/windows-media-player
cargo run
```

Use **Open File** to select your MP3/FLAC/WAV. Controls will send real commands to the Rodio backend. Audio output is initialized off the UI thread. If no audio output exists, the status bar shows an error.

```powershell
cargo check
cargo test
cargo build --release
```

**Verification caveat:** Commands above have NOT been executed in this assistant environment because Rust and a Windows runner were not available locally. GitHub Actions is configured, but no passing run has been confirmed. The code is an alpha candidate, not a tested build.

### Architecture note
We use Rodio initially to get a working playback vertical slice. Before attaching real-time equalizer and FFT, profile and choose between Rodio's `Source` filters and custom Symphonia → bounded PCM ring → CPAL. Never perform filesystem/decoder initialization or FFT work inside the real-time callback. See docs/AUDIO_ENGINE.md.

### Next work
1. Obtain a successful **Windows cargo check/test/build** and fix any Rust/Slint compilation problems.
2. Capture an audio-smoke demonstration from a real Windows host (MP3 and FLAC).
3. Add a playlist state engine, queue operations, and format metadata.
4. Implement genuinely audible 10-band EQ as a separately tested DSP component (not just draggable UI sliders).
5. Add FFT worker and visual frames only after audio latency is measured.
