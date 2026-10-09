# Modular architecture and plugin system

## Why two phases?
Premature dynamic plugin loading complicates crash safety, ABI compatibility and real-time performance. Build reusable interfaces **before** allowing untrusted code into the playback process.

## Phase A — internal modules
A versioned capability registry registers compile-time modules:
- codec adapter, audio output adapter;
- DSP node (trusted implementation, in audio engine);
- visualization renderer / analyzer (non-blocking);
- local-library metadata providers;
- download provider adapter;
- UI theme / hotkeys.

Plugin selection uses commands/events through narrow interfaces and can be feature-gated. Unit tests can substitute in-memory providers.

## Phase B — third-party extensions
- Manifest fields: id, semantic version, api_version, plugin_type, minimum app version, capabilities, permissions, publisher, integrity signature/checksum.
- Installation is explicit with path and signature/manifest validation; no automatically executable downloaded code.
- Non-real-time extensions (download sources, metadata providers, UI theme generators) run **out-of-process** with explicit resource/network/filesystem permissions when practical.
- Restrict plugin process IPC to capability-scoped request/response; timeout, cancel and crash isolation. Never pass arbitrary shell commands.
- Host may reject incompatible API versions; include migration strategy for extension settings.
- **Real-time DSP plugins** cannot naively use sandboxed RPC in the audio callback. Trusted in-process plugins require stable binary ABI, real-time-safe lifecycle and memory ownership review; VST3/CLAP hosting is optional future research, not promised MVP functionality.
- A third-party visualizer gets sampled/reduced data and may drop frames; it cannot stall cpal callback.

## Module loading contract
Discovery → dependency/permission review → initialize → activate → deactivate → shutdown → unload. Every phase has timeout/error policy. Persist per-plugin enabled state and crash count.

## Security and recovery tests
Wrong ABI, malicious manifest, invalid paths, crash during task, slow IPC, memory leak/CPU hog, attempt to read outside designated folders. Playback continues when optional modules fail or are disabled.

