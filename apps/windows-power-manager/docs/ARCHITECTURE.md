# Zilla Power Manager — Architecture

Status: v0.1 vertical slice
Profile: Appbootstrap desktop-native
Tech: Rust 2021 + Slint UI on Windows 10/11
Packaging: portable unsigned Windows x64 binary; installer deferred

## Module boundaries

- src/model.rs: side-effect-free LiFePO4 estimation formula and unit tests.
- src/power.rs: Windows powercfg adapter for reversible plan clones. Never changes original plans.
- src/displays.rs: Win32 EnumDisplayMonitors + Dxva2 DDC/CI secondary-only power adapter; no access to primary.
- src/main.rs: UI event wiring, isolated background control thread, and independent system telemetry worker.
- ui/main.slint: presentation and manual controls only. No Windows shell commands.

## Concurrency and lifecycle

A single action channel serializes changes to power schemes and monitor commands outside the UI event loop.
A second sampling worker checks CPU and RAM every 3 s and sends results through Slint's event-loop dispatch.
UI shutdown signals the workers and joins them; plan restoration runs on worker exit.
Power outages or process crashes do not guarantee restoration: explicit recovery workflow documented.

## Configuration and telemetry boundaries

v0.1 stores no secrets or BMS Bluetooth credentials. SOC and AC watts are user-provided.
Desktop Windows does not see KEMOT battery mode as a laptop power state. Future BLE reader must validate model-specific packets, report freshness and avoid automatic actions on stale readings.

## Security/safety

Normal-power actions are reversible. No privileged services, drivers or network communication.
DDC/CI commands explicitly target only non-primary monitor handles, but physical monitors may ignore power commands or become hard to wake.
Never base shutdown safety on the runtime estimate alone.

## Quality and performance

CI runs cargo fmt/clippy/test/build on Windows. Performance budget is provisional: sample 3 s; app responsiveness and standby overhead must be benchmarked on user's hardware before release.
