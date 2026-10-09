# Audio Engine 2.0 — separated decoder and bounded PCM path

**Status (2026-10-10): implementation in GitHub; not yet compiled or listened to on Windows.**

## Current path

```text
Slint UI  --transport commands--> Audio Control Thread
                                  |           |
                                  |           +-- EQ coefficients (atomic revisioned snapshot)
                                  |           +-- WASAPI device / Rodio Sink / gain fades
                                  |
                                  +-- start/cancel decoder worker on Open or Seek
                                          |
                                          +-- File::open / Symphonia decode / optional seek
                                          |
                                          +-- bounded SPSC ring (AtomicU32 f32 samples)
                                                  |
Rodio source <- BufferedPcmSource::next() <- try_pop() <-+
        |
        +-- 31-band EQ / bass / treble / loudness / preamp
        |
        +-- CPAL → Windows WASAPI
```

No disk read, decoder call or blocking wait occurs in `BufferedPcmSource::next()`. That method consumes one predecoded f32 sample from the atomic SPSC ring. If empty and the decoder is still running, it returns zero rather than stalling the mixer; when decoding finishes and the ring drains, it returns EOF. An atomic counter tracks prolonged starvation windows (256 missing samples) and appears in the player status.

## Buffering and ownership

- Producer: exactly one decode worker, performs all filesystem/codec reads.
- Consumer: one Rodio source, reads only preallocated atomic sample slots.
- Memory: requested ring size is two seconds of interleaved samples, rounded to a power of two and clamped to 4,096–1,048,576 samples (maximum ~4 MiB of sample storage per active track).
- The audio control thread waits for a bounded metadata response (up to four seconds) and at most 350 ms for prebuffering. This **never blocks the Slint UI thread**, but opening a slow file can delay a new transport command.
- Open/restart/seek create a new decoder generation; old ring is explicitly canceled after successful track replacement. Drops cancel their decoder. The reader flushes naturally by abandoning the canceled buffer, not by manipulating ring indices concurrently.
- Paused playback may fill the ring; the producer then waits with bounded sleeps. This is on its own worker only.
- Current prebuffer target is 100 ms; actual startup and underrun behavior need measurements.

## Seek and soft transitions

A seek is implemented by preparing a new decoder at the requested timestamp, then fading/pausing the old Sink and switching to the prebuffered source. Existing audio is kept if the new file/seek fails. Raised-cosine gain ramps execute on the audio **control** thread, not in the output callback. Seek changes track position base so UI reports duration correctly after restarting the decoder.

## DSP

EQ coefficients are prepared on the control thread, then published as an atomic revisioned snapshot. The mixer reads a consistent copy at most once per 128 audio frames. Per-channel Biquad state, gain envelope and output clipping protection remain on the mixer source. **The EQ is not yet acoustically validated**; future work includes frequency-response tests, noise/denormal profiling, better coefficient interpolation and limiter/gain-staging tests.

## Limits / technical debt

- Correctness of Windows compilation, Rodio/Slint API integration, audio sound quality and underrun budgets is **not yet confirmed**.
- Current transport command channel is unbounded. Rapid repeated seeking can enqueue too many decoder restarts; coalescing/cancellation remains P1 work.
- Every seek opens a fresh file decoder; seek startup is not zero-cost.
- Output latency/starvation silence can affect observed playback clock timing. Instrument decoded vs emitted frames in a future iteration.
- Variable sample-rate/channel-layout tracks are not modeled across stream spans; test and handle transitions.
- Hardware device removal/recovery, gapless playback, limiter and live FFT remain on the roadmap.
- Rust worker tests are present, but their **passing outcomes are not yet established** until Cargo CI runs.

## Verification

From `apps/windows-media-player` on Windows with Rust stable/MSVC:

```powershell
cargo check
cargo test
cargo build --release
cargo run
```

Unit tests cover ring FIFO/wraparound/cancellation/EOF, generated WAV decoding by the worker, DSP finite samples, control-thread parameter updates, playlist formats and scan subprocess logic. Manual checks must cover MP3/FLAC/WAV playing, rapid seeks, long folder scans while listening, sample rate changes, click-free fades, CPU/memory and audio device unplug.
