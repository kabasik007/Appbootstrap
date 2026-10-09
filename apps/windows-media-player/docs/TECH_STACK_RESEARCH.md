# Technology evaluation — Oct 2026

## Recommended prototype
**Rust + Slint + cpal (WASAPI on Windows) + Symphonia + own DSP graph + RustFFT + SQLite + yt-dlp subprocess**.

This is a **P0 candidate**, not a benchmark-verified winner. It optimizes for local responsiveness, high control over audio processing, Windows integration and a lightweight UI without a full browser runtime.

| Area | Candidate | Why / limitations |
|---|---|---|
| Core / services | Rust | ownership and predictable resources, native executable; team learning curve |
| Desktop UI | Slint | custom native-rendered UI, no mandatory WebView; validate complex visualizations, accessibility, license |
| Audio output | cpal + Windows WASAPI shared | cross-platform Rust API with Windows backend; control exclusive/ASIO later if needed |
| Decoding | Symphonia | pure Rust MP3/FLAC/WAV and more; verify supported format versions and gapless edge cases |
| DSP | small in-house biquad chain | explicit real-time safety, preallocation and reproducible tests; needs careful audio expertise |
| Spectral analysis | RustFFT | Rust FFT with SIMD implementations; do not run FFT on audio callback |
| Library | SQLite WAL, FTS5 (optional) | local index, querying, transactional migrations; avoid metadata I/O on UI thread |
| Downloads | yt-dlp process | large changing extractor ecosystem; process boundary, external binary/runtime/distribution obligations |
| Conversion | FFmpeg executable | optional codec postprocess; needs licensing/build audit |
| Windows JS runtime for yt-dlp | compatible Deno / other supported engine | current yt-dlp YouTube EJS needs compatible JS runtime; exact versions pinned |
| Logging | tracing or equivalent | structured asynchronous diagnostics; no callback logging |

## Alternatives
- **Rust + Tauri + React**: strong UI ecosystem, potential more flexible visual tooling, but renderer IPC and WebView memory overhead to quantify, not automatically reject.
- **C++ + Qt**: mature desktop/audio/ecosystem options, trade-offs in memory safety, licensing and toolchain complexity.
- **Rust + egui/eframe**: immediate-mode UI, flexible canvas for visualizations, potential baseline CPU/render tradeoffs; P0 spike if Slint visuals are difficult.
- **rodio**: useful for early playback prototypes. Its upstream repository notes an audio-engine rewrite in progress; for deep customizable DSP, custom Symphonia → ring → cpal path is initially preferred. Revisit based on maintainability and benchmarks.
- **Native WASAPI crate**: fallback for output capabilities not exposed through cpal, without replacing portable domain/DSP layers.

## Relevant verified upstreams / documentation
- https://github.com/RustAudio/cpal
- https://github.com/RustAudio/rodio
- https://github.com/pdeljanov/Symphonia
- https://github.com/ejmahler/RustFFT
- https://github.com/slint-ui/slint/blob/master/LICENSE.md
- https://github.com/yt-dlp/yt-dlp
- https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md
- https://github.com/yt-dlp/yt-dlp/issues/15012
- https://github.com/KenanSalar/Melodia (architecture inspiration only; AGPLv3 code cannot be casually copied into proprietary app)
- https://github.com/dcass5212/soundEQ (EQ visualization inspiration only; audit license before reuse)

## Investigation checklist before committing Cargo dependencies
- [ ] Select distribution and product code license.
- [ ] Validate Slint attribution/commercial terms.
- [ ] Confirm recent stable versions and Rust toolchain support (MSRV).
- [ ] Benchmark UI cold start and spectral drawing on target Windows.
- [ ] Benchmark cpal callback timing and Symphonia channel conversions.
- [ ] Validate exact Windows yt-dlp binary licensing and complete current YouTube dependency chain.
- [ ] Decide security/update supply-chain model.

