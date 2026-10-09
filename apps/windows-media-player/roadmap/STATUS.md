# ZillaPlayer — development status

> Generated from [roadmap.json](roadmap.json) by `tools/roadmap_sync.py`.
> Source code present does **not** mean a feature is verified or shipped.

**Snapshot:** 2026-10-10  |  **Phases:** 8  |  **Tasks:** 34

## Phase milestones

| Phase | Focus | Status | Exit gate |
| --- | --- | --- | --- |
| P0 | Toolchain & architecture | in_progress | cargo fmt --check, cargo clippy -D warnings, cargo test, Windows release EXE and playback smoke all evidenced |
| P1 | Reliable local playback | in_progress | MP3/FLAC/WAV playback, 500 seek/pause/switch loops and unplug/replug recovery |
| P2 | Advanced DSP / EQ | in_progress | Frequency response and signal tests, real hardware performance budget, no audible zipper/clicks |
| P3 | Library & playlists | in_progress | 50k files, cancel/rescan, offline devices and persisted playlists tested |
| P4 | Visualizers & themes | planned | FFT driven by real PCM, 30/60 fps caps and rendering measurements |
| P5 | Media downloader | planned | Authorized audio/video/playlist downloads, cancellation and error reporting verified |
| P6 | Extension API | planned | Version mismatch/crashing plugin safely disabled without playback dropouts |
| P7 | Release & distribution | planned | Signed installer/portable build and release smoke tests on supported Windows versions |

## Task backlog

### P0 — Toolchain & architecture

**Goal:** Get a reproducible Windows build and a defensible threading design

| ID | Priority | Task | Status | Acceptance / verification |
| --- | --- | --- | --- | --- |
| ZP-001 | blocker | Resolve Windows Rust/Slint/Rodio build and commit Cargo.lock | todo | CI Windows cargo check/test/build + lockfile |
| ZP-002 | high | Profile startup, memory, CPU and callback timing on Windows 10/11 | todo | Repeatable measurements on declared x64 hardware |
| ZP-003 | high | Create module boundaries, P0 technical ADRs and roadmap source | implemented_unverified | Review and validate repo plan JSON + documentation |
| ZP-004 | high | Check Slint, FFmpeg and yt-dlp distribution licenses | todo | Signed-off license matrix |

### P1 — Reliable local playback

**Goal:** Local playback must remain responsive during scan and seeking

| ID | Priority | Task | Status | Acceptance / verification |
| --- | --- | --- | --- | --- |
| ZP-101 | blocker | Open audio, Play/Pause/Stop, seek, master volume | implemented_unverified | Windows manual smoke + transport tests |
| ZP-102 | blocker | Replace decoder-on-callback with background producer and bounded PCM ring | todo | Callback must avoid disk/decoder work; measure underruns |
| ZP-103 | high | Coalesced transport commands and cancellation generations | todo | Stress with rapid seeks and track switching |
| ZP-104 | high | Click-free pause, stop, seek and track switch with short audio fades | implemented_unverified | Listen and analyze waveforms for clicks |
| ZP-105 | high | Detect and recover missing/changed WASAPI output devices | todo | USB/speaker device change tests |

### P2 — Advanced DSP / EQ

**Goal:** Audible, stable EQ that remains real-time safe

| ID | Priority | Task | Status | Acceptance / verification |
| --- | --- | --- | --- | --- |
| ZP-201 | high | 31-band EQ + bass/treble + preamp + bypass + loudness prototype | implemented_unverified | Rust DSP tests plus listening and frequency sweeps |
| ZP-202 | blocker | Move EQ coefficient design from callback into control worker | implemented_unverified | Control-thread coefficient publication added; still needs cargo tests, frequency sweeps and Windows callback profiling |
| ZP-203 | high | Complete editable 31-band GUI and 10-band compact preset view | todo | All bands adjustable, presets restored |
| ZP-204 | high | Gain staging, ReplayGain, limiter and clipping meter | todo | Peak/response and clipping tests |
| ZP-205 | medium | User-defined DSP presets with import/export | todo | Schema and migration tests |

### P3 — Library & playlists

**Goal:** Handle large music libraries without freezing UI or losing data

| ID | Priority | Task | Status | Acceptance / verification |
| --- | --- | --- | --- | --- |
| ZP-301 | high | Scanner subprocess, JSONL IPC and bounded UI batches | implemented_unverified | Recursive scan fixture + no blocked-process orphan tests; verify cancellation/exit and responsiveness on Windows |
| ZP-302 | high | Persist folders and playlists in SQLite with migrations | todo | Restart and migration fixture tests |
| ZP-303 | high | Virtualized searchable library, sorting and metadata tags | todo | Search under 100 ms on reference library |
| ZP-304 | medium | M3U/M3U8 import/export and proper track queue selection | todo | Unicode paths and playlist round-trips |
| ZP-305 | medium | Cover art thumbnail cache, library watch and duplicate detection | todo | Filesystem changes and memory ceiling |

### P4 — Visualizers & themes

**Goal:** Accurate responsive effects that never stall audio

| ID | Priority | Task | Status | Acceptance / verification |
| --- | --- | --- | --- | --- |
| ZP-401 | high | Nonblocking PCM telemetry tap and FFT analysis worker | todo | FFT fed by actual playback samples |
| ZP-402 | medium | Spectrum, oscilloscope, waveform, VU and peak-meter views | todo | Functional Slint visual modes |
| ZP-403 | medium | Custom skins, resizable panels, DPI and compact Winamp-like mode | todo | 125/150/200% DPI and keyboard tests |
| ZP-404 | medium | Throttle/freeze visualizer when minimized or battery-saving | todo | Idle CPU benchmark |

### P5 — Media downloader

**Goal:** Optional yt-dlp jobs isolated from playback

| ID | Priority | Task | Status | Acceptance / verification |
| --- | --- | --- | --- | --- |
| ZP-501 | high | yt-dlp/FFmpeg/JS-runtime capability probes and tool integrity | todo | Pinned versions, licensing and missing-tool errors |
| ZP-502 | high | URL analysis, playlist selection and safe format presets | todo | Preview/choose on permitted fixtures |
| ZP-503 | high | Bounded persisted job queue, retry/cancel and progress IPC | todo | Interrupt/restart/download-with-music stress |
| ZP-504 | medium | Postprocess audio, sanitize destinations and auto-import completed files | todo | File escaping, transcode and duplicate tests |

### P6 — Extension API

**Goal:** Optional plugins cannot crash music playback

| ID | Priority | Task | Status | Acceptance / verification |
| --- | --- | --- | --- | --- |
| ZP-601 | high | Versioned manifest and plugin capability schema | todo | Compatibility and invalid-manifest tests |
| ZP-602 | high | Out-of-process metadata/downloader plugin host with restricted IPC | todo | Crash/timeouts/security tests |
| ZP-603 | medium | DSP effect plugin feasibility ADR (CLAP/VST if justified) | todo | Real-time safety and license review |

### P7 — Release & distribution

**Goal:** Small, reliable installable Windows product

| ID | Priority | Task | Status | Acceptance / verification |
| --- | --- | --- | --- | --- |
| ZP-701 | high | Windows clean install/upgrade/uninstall/portable packaging | todo | Two Windows versions, update/uninstall tests |
| ZP-702 | medium | Crash diagnostics, privacy-safe logging and repair tools | todo | Failure injection and recovery |
| ZP-703 | blocker | 8-hour audio soak with scanning/EQ/downloads and user acceptance | todo | CPU/RAM/underruns and crash report |
| ZP-704 | blocker | Signed public alpha/beta and licenses/third-party notices | todo | Release artifact/signature/checksum verification |

## Quality rule

A task changes to `verified` only after its acceptance check actually ran 
with evidence (Windows build logs, reproducible test output, or measurements).
Do not fill dates or percentages by guessing; dependencies follow the P0–P7 gates.
