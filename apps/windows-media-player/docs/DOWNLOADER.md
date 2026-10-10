# Downloader module — yt-dlp adapter and job orchestration

## Intended behavior
An **optional**, self-contained download manager for authorized video/audio/playlist downloads supported by the installed yt-dlp extractors. Downloader bugs must not stop local playback.

## External tools
- **yt-dlp**: subprocess and primary URL/playlist extractor; **do not reimplement its extractors in Rust**.
- **ffmpeg + ffprobe**: separate native executables for merging audio/video and extracting/encoding audio.
- **yt-dlp-ejs + compatible JavaScript runtime** (Deno recommended in yt-dlp documentation): required for full YouTube support in contemporary yt-dlp. Record precise tested yt-dlp/yt-dlp-ejs/runtime/FFmpeg versions.
- Distribution and auto-updates need checksum/signature verification and license review; bundled Windows executables may have **different licenses** from upstream source (notably yt-dlp standalone builds and FFmpeg choices).

## High-level design

    URL from user + confirmation
      → input/url validation (http[s], size limits, no secret params in logs)
      → bounded metadata preview job (yt-dlp --dump-single-json)
      → playlist / item choice, format choice, output folder
      → persisted bounded queue (SQLite)
      → worker process (yt-dlp fixed exe path + fixed argument list)
          → progress events to parser
          → separate postprocessing with FFmpeg
      → completed file inspection and import into local library

Absolutely **no shell string concatenation** with user input. Use native process API with an array of arguments, a known executable and restricted working directory. Never use browser-cookie extraction without informed explicit user consent, and do not build DRM/access circumvention features.

## Suggested job schema
- `job_id`: UUID; `source_url`: validated and privacy-protected at rest/logs.
- `source_kind`: single / playlist; `selected_item_ids`; user intent/right acknowledgment.
- `requested_output`: best-audio-original / mp3 / opus / video-best / video-resolution.
- `destination_dir` + restricted filename template.
- `state`: new, probing, awaiting-selection, queued, running, postprocessing, completed, failed, canceling, canceled.
- `attempt_count`, `progress`, `rate`, `eta`, `last_error_code`, `output_paths`, `timestamps`.
- `archive_path`: successful item IDs used to skip duplicates via yt-dlp archive.

## UI workflow
1. Paste URL → press **Analyze**; never auto-download from clipboard.
2. Show metadata/source, whether multiple entries exist, available quality/size estimates.
3. Select specific items or range and allowed destination. Confirm start.
4. Queue shows states, progress, current file, completed/failed counts, speed, ETA, pause/cancel.
5. Finished media added to library asynchronously; partial/temp files have explicit recovery/cleanup policy.

## yt-dlp interface candidate
- Probe installation: `yt-dlp --version`; `ffmpeg -version`; optional JS runtime check.
- Preflight metadata: `yt-dlp --dump-single-json --no-download URL` (JSON may be huge for playlists; use limits/streaming selection).
- Progress: `--newline` or `--progress-template` with parseable stable prefix; verify behavior in pinned version.
- Playlist repeat sync: `--download-archive archive.txt` to avoid redownloading successful items.
- Audio extraction: `-x --audio-format mp3` when *user selects MP3* (this transcodes; metadata/artwork handled separately).
- Choose video format/quality via safe program-defined profile names, not arbitrary flag injection.
- Always pass URL as data to one final positional argument; never accept user-supplied command fragments.
- Do not hard-code credentials, authentication cookies, proxy passwords or keyring exports.

## Parallelism and CPU isolation
- Max concurrent downloads default **1** (configurable 1–3 after profiling).
- Bounded child process stdout/stderr parser and bounded state queue.
- FFmpeg transcodes must be priority/throttled relative to playback.
- Cancel gracefully then kill process tree after timeout; resumability depends on extractor/server and partial file.
- Finite exponential backoff only for transient failures; no infinite retry for 403/DRM/unsupported.

## Playlist semantics
Distinguish playlist preview, selecting ranges/items, downloading one item, and incremental re-sync via archive. Large and private lists need explicit size caps and authorization confirmation. Removing an item from local queue never deletes user originals.

## Support is not guaranteed
The upstream extractor list includes hundreds/thousands of entries and can change; generic extractor can match arbitrary URLs while failing. Do not claim "all websites" or automatic DRM-free availability. Use capability probe and transparent errors. Not every platform permits downloads under its terms even if a tool can technically retrieve content.

## Acceptance tests
- Valid permitted single video/audio with full progress output.
- Playlist selection: 3 chosen items / 50 available; only 3 queued.
- Cancel in metadata, download and postprocess; restart and preserve history.
- yt-dlp missing, FFmpeg missing, incompatible JS runtime, network timeouts, 403, 404, geo-restricted/DRM, unsupported source.
- Disallow path traversal and shell injection; filenames do not escape destination.
- 1-hour FLAC playback while download + transcode runs (monitor dropouts and UI).
- Verify licenses and update-manifest checksums before packaging.

## References (check compatibility at implementation time)
- https://github.com/yt-dlp/yt-dlp
- https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md
- https://github.com/yt-dlp/yt-dlp-wiki/blob/master/FAQ.md
- https://github.com/yt-dlp/yt-dlp/issues/15012

