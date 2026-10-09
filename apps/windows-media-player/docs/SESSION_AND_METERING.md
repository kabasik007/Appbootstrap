# Local session restore and PCM peak telemetry

**Checkpoint:** 2026-10-10. Code is present; no passing Windows build or runtime observation has been established.

## Local queue persistence

The module `src/session.rs` introduces interim **session-v1.json**, independent of the future SQLite music index.

- Storage on Windows: `%APPDATA%\\ZillaPlayer\\session-v1.json`.
- Linux development fallback: `$XDG_DATA_HOME/ZillaPlayer/session-v1.json` or `~/.local/share/ZillaPlayer/session-v1.json`.
- Schema: `{"version":1,"tracks":["absolute-or-playlist-path"],"selected":0}`.
- Maximum 50,000 entries and 16 MiB serialized payload. Paths are stored locally; no cloud account or telemetry.
- At startup, one dedicated **session I/O thread** loads the JSON file and sends it to the Slint UI as an event. The UI does **not** read the disk.
- If the user opens a track or starts scanning before the old session has loaded, their new action takes precedence.
- Queue edits cause background writes; the newest snapshot is flushed on application shutdown.
- Writes use temporary file + previous version backup; invalid primary JSON is recovered from backup when available. The worker must be tested on real Windows file permissions/antivirus behavior.
- The queue is **not automatically played** on restore. It only restores the paths and selected index.
- Metadata, indexing, tag search, multi-folder history, and migrations belong to future SQLite work (ZP-302).

## Live PCM meter (first real visualizer data)

`src/decode_worker.rs` gathers a peak envelope from **actual decoded PCM samples consumed by the output source**. It writes a peak value to the shared `PcmRing` once per 1024 samples; no mutex, heap allocation, disk operation, or FFT is done per audio sample.

The audio control worker reads a lossy atomic snapshot and publishes `audio_level_percent` in `PlaybackState`. The Slint Player shows it as a live blue **PCM peak bar**. The large spectrum graphic is still a decorative preview, **not a real FFT analyzer**. FFT processing, smoothing and VU/peak behavior per channel belong to Phase P4.

A new Rust unit test feeds a sequence of 0.65-amplitude PCM samples and checks that the meter exposes a 65% peak. This is a **code-level test**, not yet an executed test result.

## Validation checklist

1. Windows `cargo check`, `cargo test`, release build. Record output, then commit `Cargo.lock`.
2. Start with a test playlist. Close and reopen; confirm track count and selection without automatic playback.
3. Scan a large folder, close and reopen; confirm saved paths, responsiveness, and no truncated JSON.
4. Manually corrupt the primary session file; confirm backup restore and clear diagnostics.
5. Play a generated WAV or authorized music sample; verify peak bar moves with the audio signal but the spectrum remains labeled preview.
6. Measure callback sample processing cost, PCM starvation count and background disk activity.

**Roadmap:** ZP-302 (interim state written; SQLite incomplete) and ZP-401 (peak data written; FFT incomplete) remain `in_progress`, not `verified`.
