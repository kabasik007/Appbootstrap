# Audio engine — processing and playback contract

## Primary objective
Bit-perfect format decoding **before intentional DSP** (not a promise of bit-perfect output with EQ enabled), uninterrupted playback and predictable user controls.

## Proposed signal path
1. File selection / transport command.
2. Probe container and codec with Symphonia.
3. Decode in worker to PCM; validate sample rate/channels/timestamps.
4. Convert channel layout/sample format and resample **off RT callback** when device cannot accept input format.
5. Push into bounded preallocated SPSC audio buffer with generation/seek epoch.
6. WASAPI shared via cpal callback: pull buffer → gain/replaygain → EQ → optional limiter → device.
7. Tap a copy of PCM and level statistics to a bounded *droppable* analysis queue, never backpressure playback.

## Transport finite state machine
Idle → Loading → Ready → Playing ↔ Paused
Playing → Seeking → Playing/Paused
Any state → Stopping → Idle
Any active state → Error → Idle/Recovering

Race conditions to cover: seek while loading; stop during decode; open another file; default audio device changes mid-track; track deleted; end-of-track after paused; unsupported codec.

## Playback considerations
- Prefer device-supported shared mode first; optional exclusive mode later only with careful device ownership.
- Bound prebuffer to trade responsiveness against underruns. Profile actual buffer size.
- Smooth seeking and volume changes; bounded ramps to prevent clicks.
- Correct duration/timebase handling for VBR MP3 and gapless metadata where codec supports it.
- Respect output channel count; don't assume all devices are stereo.
- Audio callback never allocates, blocks on mutex, logs, waits for subprocess or calls filesystem/SQLite.
- Keep DSP parameter updates immutable or snapshot-swapped at block boundaries.

## Acceptance tests
- Decode known MP3/FLAC/WAV fixtures with expected duration/channel mapping.
- 500 repeated play/pause/seek/stop transitions on local fixtures without crashes/resource leaks.
- Device unplug/replug and default-output changes.
- Long playback while scanning library + visualizing + downloading.
- No underrun in representative smoke test; **target** eight-hour soak later, measured not presumed.

