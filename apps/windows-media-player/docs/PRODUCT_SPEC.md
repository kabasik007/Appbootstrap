# Product specification — ZillaPlayer (working name)

## Audience and positioning
Windows music enthusiasts who want a small, responsive, offline-first player with advanced sound control, quality visualizations and an optional convenient download manager. Inspired by the workflow of AIMP/Winamp, not a clone of their code, assets, branding or proprietary skins.

## Product capabilities
**Core**: fast start, play local audio MP3/FLAC/WAV, pause/seek, playlists, queue, hotkeys, drag/drop, audio output selection, theme support.

**DSP**: 10/31-band graphic EQ, parametric EQ, smooth real-time controls, level meters, presets, safe headroom/limiting, optional ReplayGain.

**Visualization**: customizable spectrum / oscilloscope / waveform, capped refresh rate, CPU-efficient rendering, pause when invisible. Prefer no video renderer at launch.

**Media Library**: metadata, artwork, search, folder watch, 50k-track target dataset (subject to tests), incremental indexing.

**Downloads**: yt-dlp-powered best effort extraction/download for sites it currently supports; video or audio, batch playlists, progress and history. Explicit permissions/rights; no DRM bypass, no unauthorized access, no guarantee every source works.

**Extensions**: built-in modules first, well-defined plugin contracts, external/untrusted adapters isolated from real-time audio.

## Non-goals for initial releases
- Full DAW, video editor, media server, social network, cloud streaming accounts.
- System-wide EQ of every Windows application.
- Automated ripping of DRM or subscription-restricted media.
- Downloading every known website with guaranteed long-term compatibility.
- Copying AIMP/Winamp proprietary UI assets.

## Primary user journeys
1. Drag an MP3 into the window → playback begins → EQ preset applied → change track with keyboard.
2. Scan local Music folder → search artist → create playlist → restore it after restart.
3. Play FLAC while spectrum visualizer runs; audio remains uninterrupted even with the main window minimized.
4. Paste a permitted media URL → preview formats/playlists → select output folder → enqueue → see exact progress or clear error.
5. Disable visualizer or download plugin without losing the active playback session.

## Accessibility and localization
Windows 10/11 x64 first. Keyboard-first navigation, high-DPI scaling, screen reader feasibility spike. Ukrainian and English initial localization; more languages later.

## Data ownership
No mandatory user account or cloud backend. Local library database and settings. Logs minimize URL/metadata retention and never store passwords/cookies by default.

