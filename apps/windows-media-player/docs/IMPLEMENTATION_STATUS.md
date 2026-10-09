# ZillaPlayer — P1 / Audio Engine 2.0 checkpoint

**Source updated: 10 October 2026. Windows build and audio playback remain UNVERIFIED.**

## Current code

- Windows-first native Rust/Slint interface (Player, Library, Downloads, Tasks, Plans).
- Separate audio **control** thread with Play/Pause/Stop, track switching, seek and master volume.
- Background **decoder worker** handles file opening, Symphonia/Rodio decoding and seeking. It feeds a preallocated, bounded, lock-free SPSC f32 PCM ring.
- Buffered audio source consumes PCM without decoding, disk I/O, sleeps, mutexes or allocation in its `next()` method.
- Prebuffer at startup, explicit cancellation when stopping/switching tracks, EOF semantics and starvation counters.
- Raised-cosine volume ramps and 31-band Biquad EQ with bass/treble/loudness/preamp/bypass. Coefficients are calculated in control worker and published through atomic revisioned snapshots.
- Library scanner is a separate supervised, lower-priority child process with JSONL IPC, bounded batches, cancel/kill/reap and shutdown joining.
- Up to 50k paths held in an in-memory queue, M3U/M3U8 import/export on background workers and first-ten preview.
- Versioned P0–P7 roadmap (34 tasks) shown in Plans and Tasks from one JSON file; source-only code never appears as verified.
- Session persistence: `src/session.rs` now asynchronously writes/restores `session-v1.json` under Windows AppData, with temp/backup recovery, a 50k-track bound, and selected queue index; it never automatically starts playback.
- Player visualizer area has a **real PCM peak bar** driven by a lossy atomic sample snapshot from the buffered output source. The large spectrum graphic remains a static mockup; FFT is not implemented.
- Corrected Rodio 0.21 file decoding type to `Decoder<BufReader<File>>` according to the upstream API.

## Tests in source (not confirmed run)

- SPSC FIFO, wraparound, concurrent producer/consumer and true EOF vs temporary starvation.
- Generated PCM WAV file decoded on an actual background worker (no copyrighted test media).
- EQ numerical checks and simultaneous atomic parameter updates.
- Scanner recursive Unicode tests / JSONL handling.
- M3U8 round-trip / oversized input.
- Volume, fade and seek input validation.
- Session Unicode restore, corrupted-primary backup recovery, invalid selection, PCM peak range and decoded-sample level tests.
- Windows Actions configuration includes `cargo check`, `cargo test`, release build and scanner-subprocess smoke.
- `tools/verify-windows.ps1` now runs check/test/release locally, checks EXE existence and reports the Cargo.lock state. Script execution is not yet confirmed.

## Not yet shipped

- Verified Windows EXE / actual audible playback / installation or startup timings.
- Coalesced rapid seeks and a generation-based transport state machine.
- Device recovery, robust gapless playback, **real FFT** (only PCM peak metering exists), complete SQLite library/indexing, all 31 EQ sliders / EQ preset storage.
- yt-dlp/FFmpeg download manager and external plugin loader.
- Stable performance limits, 8h soak test, signing and packaging.

## Current engineering gate

1. Run `pwsh -File tools/verify-windows.ps1` with Rust stable MSVC/VS Build Tools on Windows, record real check/test/release output and commit `Cargo.lock` after successful resolution.
2. Test WAV/FLAC/MP3 on a Windows audio device including pause, seek and switching.
3. Measure PCM underruns, CPU/RAM and device handling. Resolve any decoder or UI build/runtime errors.
4. Expand partial adjacent-seek coalescing into fully cancellable transport generations; implement the SQLite index and FFT worker after Windows playback verification.

No "all tests passed" claim exists until the Windows runner actually produces results.
