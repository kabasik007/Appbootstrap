# Performance budget and verification plan

**All values below are proposed engineering TARGETS, not observed benchmarks or guarantees.** Complete real baselines in Phase 0, then tune thresholds using specified hardware/builds.

## Reference configuration (proposal)
- Windows 10/11 x64
- Mid-range quad-core laptop/desktop from approx. 2019+, 8 GB RAM, SSD
- Windows display 1920×1080, 100% and 150% DPI
- Audio: integrated sound WASAPI shared, 44.1/48 kHz stereo
- Library: 50,000 media file metadata records, artwork cache; 3 playlists
- Audio test: representative MP3 VBR/CBR, WAV PCM, FLAC; legal redistributable fixtures
- All measurements from **release** builds. Repeat tests cold and warm.

## Initial proposed budgets
| Metric | Aspirational threshold | Evidence required |
|---|---|---|
| Cold launch to interactive UI | p95 <= 2.5 s | Process + UI input trace (10+ runs) |
| Warm launch to interactive UI | p95 <= 1.0 s | Same machine/cache policy |
| App RAM idle, indexed library | <= 120 MiB | Private working set/working set documented |
| Playback CPU (no vis/download) | <= 3% aggregate CPU on reference machine | 10-minute stable sample with sample rate |
| Pause/play command response | p95 <= 100 ms | Instrument transport → output transition |
| UI navigation under scanning | p95 <= 120 ms | Input event traces under background load |
| Visualization | 60 FPS on reference GPU; cap 30 FPS fallback | Frame timing and dropped frame count |
| Audible underruns | 0 in 8 h stress playback target | Callback underrun counter + manual listening |
| Bounded background queue | No unbounded memory growth | Stress + soak metrics |
| Download concurrent tasks | Default 1, max configurable after tests | Playback priority + CPU observation |

Thresholds must be confirmed or updated once we have a prototype. Do not treat the app as meeting these goals before real measurements.

## Repeatable test scenarios
1. **Startup**: cold 10 runs, warm 10 runs, DPI and antimalware variance recorded.
2. **Seek storm**: 500 rapid seek/play/pause/open requests; no race/crash/leak.
3. **Audio stress**: 8 h play with EQ, visualization, database scan and yt-dlp job; track overruns/underruns.
4. **Library stress**: 50k files with corrupt tags, long Unicode paths, covers >5MB, disconnected storage.
5. **UI stress**: resize, minimize, Windows scale 150%, drag playlist 1k tracks, hotkeys.
6. **Downloader stress**: unsupported URL, network timeout, 1k entry playlist preview with caps, FFmpeg exit failure.
7. **Cleanup**: repeated reopen and window close; no leaked subprocesses, device handles or orphan temp files.

## Profiling tools (candidates)
- Windows Performance Recorder / Windows Performance Analyzer or ETW
- Rust `tracing` and `cargo flamegraph` where platform/toolchain supports it
- Process Explorer / Windows Task Manager for memory/CPU samples
- Audio engine counters: buffer occupancy, callback duration, decode throughput and missed deadlines
- Avoid high-frequency logging from real-time callback; aggregate counters asynchronously.

## Continuous regression checks
CI checks format, clippy, unit and smoke tests after code exists. Hardware benchmarks should run on controlled Windows machines, not arbitrary GitHub-hosted runners as release claims.

