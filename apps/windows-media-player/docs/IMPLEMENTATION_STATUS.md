# ZillaPlayer implementation checkpoint — 10 October 2026

**Code has been committed. A Windows build, audio playback and benchmarks have NOT been verified.**

## Added since the original UI prototype

- Folder scanner launched as a **separate child process** of the player (flag: --scan-worker). It does not initialize the GUI or audio. On Windows the scanner has below-normal process priority.
- Scanner stdout is JSON Lines. A dedicated reader thread sends bounded chunks of 64 tracks into a bounded 64-event UI queue; generation checks cancel outdated scans.
- The main thread stays responsive and only renders the first ten file names. In-memory playlist domain supports up to 50,000 paths and Next/Previous/Select.
- A Rust audio control worker connects Rodio/CPAL playback and message-driven commands to Slint.
- Pausing, switching tracks and seeking fade down then up with a 50–70 ms raised-cosine envelope. The sleeping/ramp code executes on the **control thread**, never directly in a Slint callback.
- 31-band graphic EQ filter chain with ±12 dB, bass shelf, treble shelf, loudness switch, EQ bypass, preamp headroom and per-channel Biquad state. The compact UI exposes 15 of the 31 frequency bands.
- Added targeted unit tests for playlist wraparound, recursive Unicode folder scanning and JSON Lines IPC, DSP finite output/coefficient validity, concurrent EQ snapshot updates, and fade monotonicity. Windows CI now includes a scanner subprocess smoke test (results not yet confirmed).

## Important limitations and technical debt

1. NO proof of successful Windows compilation or real MP3 playback: CI status and Windows host execution must be checked before calling this build ready.
2. The existing Rodio decoder can still perform sample decoding during audio output callbacks. The audio **control** thread alone does not fully isolate decoder I/O. A pre-decoding producer thread and bounded PCM buffer are required next.
3. EQ filter coefficients are now prepared by the audio CONTROL thread and published through an atomic revisioned snapshot. The audio path copies only stable coefficients at most once per 128 frames. The callback still may decode through Rodio and requires profiling; this is **not** proof of full real-time safety.
4. The scanner runs in its own process under a supervisor using nonblocking `try_wait()` and an independent stdout reader. Cancellation kills and reaps the process; shutdown joins supervisors. Windows stress tests and profiling remain pending.
5. In-memory list only. Saved playlists, folder history, SQLite index, album artwork, full 31-band editing panel and preset persistence are not finished.
6. Loudness is an initial compensation contour, not yet calibrated or volume-dependent.
7. Spectrum visualizer is currently **decorative**, not driven by live FFT. Downloader/Tasks/Plans are UI prototypes.
8. Supported audio file formats must be tested on Windows with actual fixtures.

## Local Windows verification

Requires Rust stable MSVC, Visual Studio 2022 Build Tools (C++ and Windows SDK). From apps/windows-media-player run:

    cargo check
    cargo test
    cargo run
    cargo build --release

Manual checks: open MP3/FLAC/WAV, pause/resume/stop, repeated seeks and volume changes, EQ/bass/treble/loudness effect, scan nested directories, change folders mid-scan, next/previous playback, close while scanning, disabled audio device, and RAM/CPU sampling.

**Next engineering gate:** obtain passing Windows compilation, scanner subprocess smoke results and real listening measurements. EQ coefficient design is now moved to the control worker, but the Rodio source decode path must still be decoupled using a bounded PCM ring before the engine can be described as hard real-time safe.
