# DSP, advanced equalizer and visualizations

## EQ architecture
- Single deterministic DSP graph running on interleaved/planar f32 blocks chosen by P0 spike.
- Modules: preamp/headroom → optional ReplayGain → graphic EQ → parametric EQ → balance → optional soft limiter → output.
- **Graphic EQ**: start with 10 bands, later 31 bands. Use biquad peaking/shelving shapes as suitable; verify aggregate response and Q/inter-band behavior.
- **Parametric EQ**: configurable frequency, gain, Q/slope; 8–12 bands proposed.
- Smooth coefficient updates/short interpolation to prevent zipper noise/clicks; test rapid preset changes.
- Gain staging required to avoid clipping from positive EQ boosts. Provide visual clipping indicator.
- Presets saved as schema-versioned user settings with safe fallback.

## Real-time constraints
- Prebuild coefficients and graph state outside callback; publish bounded immutable/atomic parameter snapshot updates.
- Allocate/filter state per channel in initialization; no per-sample heap operations or blocking synchronization.
- Test with reference PCM vectors, impulse/frequency sweeps, denormals, NaN/inf and extreme Q/gain/volume changes.
- Benchmark on Windows release builds and representative audio hardware. DSP quality is a measurable audio-technical property, not subjective claims alone.

## Visualizer architecture
- Audio callback sends a bounded *best-effort* snapshot / ring to analyzer; never waits for it.
- Analysis worker applies Hann windows and RustFFT (candidate) for 2048/4096 point spectra, peak/RMS, smoothing and log-frequency aggregation.
- UI pulls/receives reduced-size frames at <=30/60 FPS. If minimized/hidden, pause or throttle analysis/rendering.
- Modes: spectrum columns, spectrum line, oscilloscope, waveform overview, peak/VU meter. Optional GPU rendering after profiling.
- Effect control and visual rendering remain independent: crash or overload in visualizer must not interrupt playback.

## Future: audio effects
Compressor, channel mixer, convolution reverb, crossfade, optional advanced plugin hosting are backlog only. No giant DSP dependency introduced before profiling and licensing assessment.

