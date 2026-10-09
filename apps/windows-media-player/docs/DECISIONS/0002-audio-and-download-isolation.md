# ADR-0002 — Prioritize deterministic audio; isolate downloads

- Status: Proposed (to validate in Phase 0)
- Date: 2026-10-10

## Context
The user wants both responsive media playback with DSP/FFT and an integrated multi-site download manager. Media extraction/transcoding consumes CPU, disk and network resources unpredictably.

## Decision proposal
Use a dedicated cpal/WASAPI audio callback, worker-based decoding with bounded PCM ring, DSP preallocated per output format, and separate droppable FFT analysis. Manage yt-dlp and FFmpeg through supervised subprocesses with bounded concurrency, process cancellation, rate-limited progress updates and no shared critical locks with audio.

## Alternatives
- One unified media runtime: simpler initially but puts playback at risk.
- Running FFT/decoder/network on output callback: rejected due to glitch risk.
- Reimplementing yt-dlp extractors in Rust: rejected as high maintenance burden.

## Validation
Verify locally on Windows with continuous FLAC playback while library indexing and authorized test downloads run, measuring callback timing, CPU, memory, dropouts and UI p95. No claims about results until tests have run.

