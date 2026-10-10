# Zilla Studio — training / virtual MIDI keyboard (Alpha)

**Release scope:** Prototype of a musical practice workstation. This is **not** a copy of FL Studio or a full DAW.

## What is implemented

- Separate **Studio / Студія** page in the native Windows application.
- Two clickable octaves **C4–B5** (24 MIDI pitches 60–83).
- Rust procedural synth, four instrumental timbres: **Piano, Bass, Lead, Pad**.
- Polyphonic notes; multiple note sources can mix concurrently.
- Four-channel **live mixer**: Gain, pan, mute. Separate master volume.
- Adjustable **40–240 BPM click metronome**.
- Simple C-major scale trainer (**C4 D4 E4 F4 G4 A4 B4 C5**). Correct notes advance the lesson; misses count in attempts; reset available.
- Synthesizer processing uses a separate Rodio audio stream and worker and does not decode media on the UI thread.
- Live channel controls are lock-free atomics at the sound sample stage; no heap allocation on audio callbacks.

## Quick start

1. Launch ZillaPlayer and open **♬ Студія / Studio**.
2. Select one of four instruments.
3. Click any note on the virtual keyboard. You should hear a short synthesized note.
4. Adjust gain, stereo pan and mute on the mixer. Master affects all Studio voices, **not** local music playback.
5. Turn on Metronome, adjust BPM in steps of 10.
6. Play the highlighted target note in the C-major training section. Follow the next highlighted target and compare hit score.

## Deliberate boundaries

- The keys **represent MIDI note pitches** but **do not yet support USB MIDI controllers**. Hardware input discovery, velocity, Note Off, sustain pedal and hotplug are P8 follow-ups.
- No piano-roll editor, MIDI export, recording, VST, loops, patterns, effect chains or full DAW sequencing yet.
- Each click plays a short envelope-shaped note. Voice lifetime, fixed harmonics and timbres are not sampled piano libraries.
- No user session persistence for the Studio mixer yet. Levels and BPM reset when the app starts.
- Real hardware audio output (including two simultaneous Rodio streams for the music player and Studio) must be tested manually on Windows. If the audio device cannot open, notes may be inaudible.
- Unit tests validate generated note finiteness, pitch mapping, note lifetimes and trainer scoring. They **do not** prove the actual speakers/driver work.

## Next milestone priorities

1. Device list and USB-MIDI connect/disconnect via a separate WinMM/midir worker.
2. Key presses and Note Off with ADSR sustain/release, velocity sensitivity and sustain pedal.
3. Virtual keyboard key mapping (QWERTY / AZERTY), octave controls, visual played-note highlight.
4. 16-step piano roll, recording with timestamps, playback and standard MIDI-file export.
5. Combined audio routing with the existing ZillaPlayer output, latency measurement and crash-safe project save.

Track in [P8 Studio & MIDI practice](../roadmap/roadmap.json). 
