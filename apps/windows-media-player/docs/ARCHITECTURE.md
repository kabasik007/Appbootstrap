# ZillaPlayer — architecture v0.2 (implemented vs target)

> **The flow diagram below describes the target architecture, not what the current alpha has already verified.** Current source: Slint UI → background control thread → Rodio Decoder/Source → EQ → CPAL/WASAPI. Folder scanning is a separate supervised child process with bounded JSONL IPC. Coefficients are now prepared on the control thread and published via revisioned atomics. The **PCM decode producer/ring and FFT analyzer are still pending**, as are Windows build and audio performance measurements. See [implementation status](IMPLEMENTATION_STATUS.md).

## Cross-cutting goals
- Sound playback is a higher-priority workload than indexing, drawing or downloading.
- In-process real-time audio callback has no blocking locks, allocations, database, network or decoder initialization.
- UI, worker services and external downloader communicate via bounded messages and observable state.
- User can disable downloader and visualizers: local playback still works.

## Target runtime diagram (future work)

    Slint UI / input thread
           |  asynchronous commands / state snapshots
           v
    app-service (transport FSM, playlist, queue, preferences)
      |           |                        |
      v           v                        v
    playback   library service        download manager
      |        SQLite + scan          yt-dlp subprocess
      v        worker + watch          FFmpeg / EJS runtime
    decode workers
    (Symphonia)
      |
    bounded PCM ring buffer <-- cancellation / seek epoch
      |
      v
    cpal WASAPI output callback (real-time)
      |
    gain -> EQ -> optional limiter -> device
      |                  ^
      +-- nonblocking copy/snapshot --> FFT/VU analysis worker
                                         |
                                    capped-rate snapshots
                                         |
                                      visual UI

    plugin registry: built-ins initially
      \-- isolated extension hosts later (non-real-time only)

## Suggested Cargo workspace (do not create empty crates without code)

    apps/windows-media-player/
      crates/
        app/           Windows entrypoint + Slint UI binding
        domain/        Track, playlist, settings, commands, transport types
        playback/      transport FSM, scheduling, seek, prebuffer
        decoder/       Symphonia adapters, format probing
        audio-io/      cpal WASAPI/device discovery/output
        dsp/           biquads, EQ graph, limiter, meters
        analysis/      FFT + visualizer frames, off audio thread
        library/       SQLite schema, metadata, artwork, indexing
        downloads/     managed yt-dlp subprocess and queue
        plugins/       capability registry + versioned manifests
      tests/
        fixtures/      only redistributable legal media fixtures

This is a **candidate** component decomposition. During P1, combine crates if compilation/review overhead outweighs boundary value.

## Core contracts
- `PlaybackCommand`: Open, Play, Pause, Stop, Seek, Next, Previous, SetGain, SetEQ, SetOutput.
- `PlaybackEvent`: Loading, Ready, Playing, Paused, Position, BufferUnderrun, DeviceChanged, RecoverableError, FatalError.
- `AudioBlock`: PCM f32 interleaved/planar format agreed once, channels/sample rate, fixed frames, monotonic generation/seek epoch.
- `LibraryQuery` and `TrackId`: persisted index decoupled from audio rendering.
- `DownloadJob`: source URL, output format, destination, playlist selection, owner authorization acknowledgement, lifecycle.
- `ExtensionManifest`: plugin ID, API version, type, permissions, execution isolation and publisher identity.

## Concurrency / ownership (target contract, incomplete in alpha)
- Target: output callback exclusively consumes prebuffered blocks and owns its DSP state, with no await/logging/DB calls. **Not yet met**: current Rodio source may decode during mixing; a bounded PCM producer/ring is next.
- Decoder worker performs file I/O and decoding; seek cancels old generation and flushes staging buffers before activating new epoch.
- Library scanner uses bounded queue / bulk transaction and must yield to playback.
- Visualization worker reads its own bounded lossy ring; dropping frames is acceptable, dropping/late audio is not.
- Downloader manages child processes and bounded concurrency, with cancel/kill/retry and resource limits.
- UI only subscribes to compact state snapshots and sends commands; never holds the audio callback lock.

## Fault handling
- Missing device: stop/switch with recoverable state and user-facing notification.
- Corrupt media: skip/stop with error and full cleanup.
- Plugin crash: disable plugin, keep playback alive.
- yt-dlp unsupported/error/changed extractor: mark job failed with actionable output, not crash UI.
- No silent retries; backoff and finite limits.

## Security
All downloaded paths are sanitized and normalized; never pass unsanitized user URL to a shell, use argument vectors to spawn **only pinned executables**. No implicit browser-cookie extraction, credential harvesting or bypass behavior. Confirm redistribution rights for all bundled components.

## Decision records needed before implementation
- Rust/Slint license and render benchmark vs native alternatives.
- cpal or WASAPI layer abstraction, backend fallback.
- Audio block format and lock-free queue approach.
- Plugin process boundary and eventual DSP extension strategy.
- yt-dlp / FFmpeg / Deno bundling vs user-installed dependencies.

