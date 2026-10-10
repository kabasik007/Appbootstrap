# Live FFT Visualizer — P4 first functional implementation

**Checkpoint:** 10 October 2026. Code is in the GitHub branch; Windows compilation, audio playback and visual frame timing are NOT yet independently verified.

## Signal path

```text
File → decoder worker → bounded PCM playback ring
                               |
                               v
                BufferedPcmSource::next() → 31-band DSP → CPAL/WASAPI
                               |
                               +--- lightweight sample tap (first channel)
                                      |
                                      v
                           bounded SPSC analysis ring
                                      |
                         separate FFT analyzer THREAD
                                      |
                 Hann 2048 samples + RustFFT + 1024-sample hop
                                      |
                    32 logarithmically spaced frequency bins
                                      |
                  atomic Spectrum snapshot, generation guarded
                                      |
                   Slint VecModel<f32>, 20 FPS UI timer
                                      |
                          dynamic animated spectrum bars
```

The large spectrum is now wired to *real* audio sample data. Earlier versions showed a decorative hardcoded array; that has been replaced.

## Performance isolation

- **Never run FFT on the audio/mixer thread.** The audio source only makes a nonblocking SPSC `try_push` call for one sample per multichannel frame.
- If the analyzer cannot keep up, the analysis buffer fills and **new visualizer samples are dropped**, not blocking the sound output.
- FFT window: 2048 mono samples, Hann; hop 1024; analysis update rate at 44.1 kHz is at most ~43 FFTs/second before throttled UI rendering.
- UI redraw model is updated at most once every 50 ms (~20 FPS), and only for bins changing by more than 0.75 percentage points. This is an initial power-saving limit, not a measured guarantee.
- 32 display bands cover approximately 32 Hz–16 kHz logarithmically. Bands above Nyquist are zeroed for lower sample rates.
- dB-to-height mapping is a first-pass relative display (−75 dBFS to 0 dBFS). It is *not* a calibrated sound-pressure meter.
- Each new track/seek has a new generation; outdated FFT workers cannot overwrite values for current playback.
- Workspaces, FFT plan, Hann window and scratch buffers are allocated only on the analysis worker.
- Resource/thread cleanup: the tap cancels the analysis ring on source drop, and analyzer exits when the active track ends or generation changes. The thread is detached; if this proves unreliable in stress tests, introduce explicit join ownership.

## Current limitations

- The v0.1 visualizer taps **channel 1 (first channel)** to avoid channel mixing on the output callback. Audio output still handles all channels. Future versions should downmix stereo/multichannel safely with per-channel energy meters.
- Analysis samples are copied ahead of EQ. The graph reflects decoded signal, **not post-EQ output**, while the separate PCM peak meter also samples before EQ. Change tap point once DSP and limiter metering are stabilized.
- No oscilloscope, VU meter, waveform editor, album covers or user-defined visualizer plugins yet.
- No minimized-window/battery-saving throttling yet beyond 20 FPS UI cap.
- Frequency bins and smoothing are heuristic. Verify with known-frequency test tones, varied rates, silence, clipping and actual listening.
- Hardware stress measurement is essential: compare CPU and latency with visualizer enabled/disabled and with concurrent 50k-file scan.

## Tests included (unverified until Cargo runs)

- 1 kHz generated sine produces a high band near its frequency.
- Silence produces zero bars, extreme input levels produce bounded/finite bars.
- Old generations cannot overwrite a new track's spectrum.
- Visualizer producer drops analysis data rather than blocking when buffer is full.
- Existing SPSC FIFO and generated WAV decoder tests remain relevant.

## Next steps

1. Get a successful Windows `cargo check`, `cargo test`, Release build and lockfile.
2. Confirm a real MP3/FLAC/WAV produces moving FFT bars and no audible dropouts.
3. Add stereo downmix, frequency axis labels, oscilloscope and VU/peak history.
4. Dynamically cap analysis rate when minimized; target stable 30/60 FPS options only after profiling.
5. Add color palettes, visualization presets and reconfigurable modules later.

**Milestones:** ZP-401 `implemented_unverified`, ZP-402 `in_progress`. P4 is not a finished/shipped phase.
