# ZillaPlayer — Engineering Roadmap

**Версія плану:** 0.1 · **Орієнтир:** Windows 10/11 x64 · **Статус:** запропонований, без фіксованих календарних дат.

## Принципи порядку розробки

Спершу отримати **стабільний локальний аудіоплеєр**, потім професійні DSP і візуалізації, а вже після цього — downloader та зовнішні плагіни. «Комбайн» не повинен ламати просту дію Play/Pause. Кожна фаза дає робочий і перевірний інкремент.

## Phase 0 — Архітектура та proof-of-concept (P0)
**Результат:** обрана технічна основа і відтворюваний Windows build.

- [ ] Підтвердити цільові Windows 10/11 x64, min CPU/RAM, інсталятор і політику ліцензії.
- [ ] Встановити стабільний Rust toolchain + MSVC, зафіксувати версії, `Cargo.lock`.
- [ ] Spike: Slint UI, DPI scaling, custom waveform/spectrum drawing, accessibility, license.
- [ ] Spike: cpal/WASAPI shared; MP3/FLAC/WAV decoding Symphonia; seek, device switch, pause.
- [ ] Spike: DSP biquad + preamp у callback без алокацій; пасивний FFT worker.
- [ ] Spike: окремий subprocess yt-dlp на Windows (версія, --version, ffmpeg, EJS/JS runtime discovery); **без автоматичного скачування**.
- [ ] Узгодити ADR про stack, plugin extension model і правовий спосіб постачання залежностей.
- [ ] Налаштувати Windows CI: fmt, clippy, unit + build + smoke (коли існуватиме код).
- [ ] Створити репрезентативний набір legal/local media test fixtures.
**Gate:** аудіо spike працює на справжній Windows-машині, документація містить реальні build команди і виміряні базові показники.

## Phase 1 — Minimal Player (MVP core)
**Результат:** стабільна програма «відкрив файл → грає».

- [ ] Cargo workspace (app, core, audio-output, decoder, domain, tests); залежності через інтерфейси.
- [ ] Невелике вікно: Open file, Play, Pause, Stop, Seek, Volume, Next/Previous.
- [ ] MP3, WAV, FLAC: decode worker → bounded prebuffer/ring → output; sample rate/channel conversion when necessary.
- [ ] States: Idle, Loading, Playing, Paused, Seeking, Stopping, Error; recover from bad file/device unplug.
- [ ] Фонове читання, cancel під час seek/open, graceful shutdown, базове логування.
- [ ] Unit/integration tests, Windows release build.
**Gate:** без чутних збоїв на тестових локальних файлах; жодних блокувань UI при відкритті/перемиканні треків.

## Phase 2 — Advanced DSP / Equalizer
**Результат:** якісне керування звуком, без стрибків гучності.

- [ ] Preamp/headroom, volume, ReplayGain normalization policy, bypass.
- [ ] Graphic EQ: 10-band first, 31-band as optional advanced view.
- [ ] Parametric EQ: 8–12 configurable bands (peak/shelf/high-pass/low-pass).
- [ ] Smooth parameter changes, coefficient interpolation/crossfade avoiding zipper noise.
- [ ] EQ presets + save/load + per-device/per-output settings.
- [ ] Stereo balance, optional soft limiter, peak/RMS meters; clipping indicator.
- [ ] DSP benchmark CPU usage and impulse/frequency-response tests.
**Gate:** deterministic DSP tests, no audible clicks when changing presets under normal conditions, no callback allocation/lock violations.

## Phase 3 — Library, playlists, metadata
**Результат:** швидка робота з великими музичними колекціями.

- [ ] Recursive folder scan as bounded background work; incremental updates and cancellation.
- [ ] ID3/Vorbis/FLAC metadata, artwork thumbnail cache, UTF-8/Unicode, malformed tags.
- [ ] SQLite schema + migration policy, WAL / FTS5 where helpful.
- [ ] Playlists/queue: reorder, shuffle, repeat, history, M3U/M3U8 import/export.
- [ ] Search/filter/sort, duplicate detection rules; avoid scanning whole disk on launch.
- [ ] Session restore, optional portable mode.
**Gate:** opening the UI stays responsive while a 50k-track library is scanned; benchmark and error report recorded.

## Phase 4 — Visualizers, skins, hotkeys
**Результат:** AIMP/Winamp-style experience, optional visual effects.

- [ ] Spectrum analyzer (FFT 2048/4096, frequency grouping, smoothing, logarithmic scale).
- [ ] Oscilloscope, waveform overview, level/VU meters.
- [ ] 30/60 FPS visualization caps, no redraw when hidden, pause on minimize/eco mode.
- [ ] Theme tokens, classic compact layout, modern wide layout, scalable DPI, accessibility.
- [ ] Multimedia keys, hotkeys, drag/drop, system tray; dark/light themes.
- [ ] Visualizer failures must not affect audio playback.
**Gate:** audio uninterrupted when visualization is turned on/off; UI performance profiled on baseline hardware.

## Phase 5 — Downloader (yt-dlp integration)
**Результат:** окремий менеджер завантаження відео, аудіо й дозволених плейлистів.

- [ ] Managed yt-dlp process adapter; detect/manage compatible yt-dlp, FFmpeg, optional JS runtime/EJS dependencies.
- [ ] URL paste → metadata/playlist preview → select individual items/range → consent → queue.
- [ ] Formats: original best-audio, extract MP3/FLAC where transcoding is suitable, video profiles/resolutions.
- [ ] Path templates, safe filenames, collision policy, destination choice, disk-space checks.
- [ ] Download manager: queue, pause via cancel/resume where supported, progress, speed/ETA, stop, retry, archive to skip duplicates.
- [ ] Download playlists and channels within permissions/capabilities and platform policies.
- [ ] Add successfully completed files to media library asynchronously; never on audio thread.
- [ ] Site support is **best effort**; supported extractor list is not a guarantee of function.
**Gate:** download/cancel/restart tests and UI responsiveness while playing FLAC; clear error messages for unsupported/restricted sources.

## Phase 6 — Plug-in APIs, advanced audio
**Результат:** стабільні точки розширення без нестабільного ABI.

- [ ] Manifests, semantic API versions, feature/permissions flags, per-plugin disable and crash diagnostics.
- [ ] External process plugin adapters for downloader providers / metadata / skins / visualization (isolate untrusted modules).
- [ ] Trusted in-process DSP plugins only after real-time safety review; optional CLAP/VST host exploration as **separate ADR**.
- [ ] Gapless/crossfade quality, replaygain edge cases, outputs and exclusive WASAPI/ASIO optional spike.
**Gate:** version mismatch rejected safely; external plugin crash does not crash player.

## Phase 7 — Beta / packaging / release
**Результат:** installer, support and measured stability.

- [ ] Windows 10/11 x64 clean install, upgrade/uninstall and optional portable zip.
- [ ] Signing/update strategy; license notices and third-party binary distribution audit.
- [ ] Crash report opt-in, privacy-safe logs, recovery, migration and backup/export.
- [ ] Profiling on baseline and low-end devices; multi-hour playback with downloads/visualizations.
- [ ] Accessibility, Ukrainian/English UI, keyboard and media-key testing.
**Gate:** tests and release checklist complete, known issues published. A signed binary cannot be claimed before signing is implemented.

## Priority summary

| Release milestone | Mandatory outcome | Dependencies |
|---|---|---|
| P0 | Audio + UI + downloader feasibility, Windows toolchain | none |
| P1 | Play local MP3/FLAC/WAV | P0 |
| P2 | EQ/DSP | P1 |
| P3 | Library/playlists | P1 |
| P4 | Visualizers / themes | P2 |
| P5 | yt-dlp download manager | P1 and isolation contracts |
| P6 | Third-party plugins | P2, P4, P5 contracts stabilized |
| P7 | Distributable beta | stability across phases |

## Exit criteria: first public MVP
- Working Windows executable from clean checkout.
- Local playback MP3/WAV/FLAC with pause/seek/volume, test coverage and error messages.
- UI responsive and measured on declared hardware.
- Downloader is **not** required for first MVP; its development is scheduled after stable playback.

## Top project risks
1. Audio glitching due to locks/allocations/device changes in callback.
2. Feature creep before stable playable MVP.
3. Slint licensing and custom visuals complexity.
4. yt-dlp/YouTube extractor changes and EJS/FFmpeg packaging/licensing.
5. Plugins destabilizing runtime, untrusted downloads and installer security.
6. Unsupported codecs or gapless seeking behavior.
7. Performance numbers promised without reproducible measurement.

All estimates/budgets are provisional until P0 benchmarks exist.

