# Audio engine — evolving implementation

## Current P1 alpha
The first playable implementation uses **Rodio 0.21.1**, which builds on CPAL for device playback and Symphonia for default codecs. A dedicated Rust control thread receives Slint commands over `std::sync::mpsc`; a bounded status channel with `try_send` sends UI snapshots at 200 ms intervals.

Current path:

```
Slint UI → mpsc Commands → audio control worker
                             │ File::open + rodio::Decoder (Symphonia)
                             │ Sink (play/pause/stop/seek/volume)
                             └→ OutputStreamBuilder → CPAL → WASAPI
Slint UI ← bounded mpsc snapshot channel ← worker
```

The current alpha does **not** yet expose raw PCM blocks or any working DSP/FFT tap. It must not be marketed as a custom low-latency audio engine until validated.

## Planned P2–P4 custom DSP topology

1. File selection / transport command.
2. Probe container and codec with Symphonia off UI thread.
3. Decode worker produces PCM; validate sample rate, channel layout, timestamps.
4. Convert channel layout/sample format and resample off time-critical callback.
5. Push blocks into a bounded, preallocated SPSC buffer with generation/seek epoch.
6. cpal/WASAPI callback pulls from buffer → volume/gain → preallocated EQ → limiter → device.
7. Tap compact PCM/VU snapshots into a **droppable** ring to the FFT worker.

## Transport state and lifetime
Idle → Playing ↔ Paused → Stopped. Open another valid file atomically replaces current track; an invalid file is rejected while current track continues. Stop retains a restartable file path in process memory.

Store all output stream and sink handles within the control worker and ensure they remain alive while playing. Only documented control operations are allowed across the UI thread; closing the UI terminates the command channel and audio worker.

## Pending engineering validation
- Make rustc/cargo Windows build pass with pinned dependencies and checked lockfile.
- Exercise MP3 VBR, FLAC, WAV, seek, bad file, device absent, unplug/replug, repeated play/pause.
- Benchmark callback deadlines, CPU, memory and dropped buffers on target hardware.
- Add rate/channel conversion, true gapless timing, device-change recovery and prebuffer tuning.
- Separate visualizer performance from the audio path.

## Acceptance (not yet met)
- 500 repeated play/pause/seek/stop transitions and long playback without crashes.
- Continuous output on stable test hardware during UI interactions, library indexing and downloads.
- No allocations/locks in any future custom real-time callbacks.
- All claims must be supported with actual benchmark logs.
