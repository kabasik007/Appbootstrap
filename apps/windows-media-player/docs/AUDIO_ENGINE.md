# Audio engine — current and target

## Current P1/P2 code (unverified on Windows)

Slint UI sends commands to an audio CONTROL worker using mpsc. That worker owns Rodio Sink/OutputStream handles, performs user-requested pause/resume, stop, local file open and seek, and applies raised-cosine fades (about 50–70 ms) to avoid abrupt changes. A bounded channel sends playback snapshots back to the Slint UI timer.

Audio source wraps Rodio Decoder in the ZillaPlayer EqSource adapter. EQ uses 31 peaking bands with per-channel state and smoothed coefficient changes, bass/treble shelves, a loudness contour and an optional bypass. Main UI controls 15 of the 31 EQ bands; 31-band advanced editor and saved presets remain pending.

## Important hard real-time caveat

The existing Rodio Source is still consumed on the audio mixing/output path. Decoding or I/O *might* happen during output callbacks: the control worker only isolates device setup and transport commands. EQ parameter refresh now copies prepared atomic coefficients; trigonometric coefficient generation runs on the control thread. Do not claim this is completely non-blocking or hard RT safe.

## Target after Windows build is verified

    Slint → asynchronous control commands
               ↓
    decode worker with disk reads and seek generation
               ↓
    preallocated bounded SPSC PCM ring
               ↓
    CPAL/WASAPI output callback (no allocations, blocking I/O or mutexes)
               ↓
    prebuilt EQ coeff snapshots + gain envelope → output
               └→ loss-tolerant FFT analysis worker → throttled UI

Filter coefficient creation has now been moved to the audio-control worker and shared through an atomic revisioned snapshot; the decoder still needs isolation before the callback can be considered real-time-safe. Test seek cancellation and stale data discard with generation counters. Measure underruns, latency, memory, CPU and concurrent folder scanning on a real Windows system.

## Tests needed before release

- Windows cargo check, cargo test, release build and MP3/FLAC/WAV fixtures
- 500 rapid seek/pause/track transitions
- Eight-hour playback soak with 50k music files being indexed
- Device unplug/replug, corrupt media and invalid/remote folder
- Smooth audible fade without clicks and real effect from every 31-band setting
