# Roadmap

## v0.1 — manual policy (in implementation)
- Rust+Slint UI, manual SOC/watt estimate, CPU/RAM sampling
- Normal/Eco/Emergency reversible Windows power plans
- Secondary-only DDC/CI manual sleep/wake and recovery
- Windows CI + unit tests

## v0.2 — metrics
- Detect CPU/GPU model and GPU vendor adapter where supported
- Read-only JK BMS Bluetooth data: SOC, voltage, current, cell metrics, temperatures
- Disconnect/reconnect, stale data, no writes to BMS
- Optional wall AC wattmeter calibration and runtime prediction from battery DC watts

## v0.3 — safe automation
- Sustained discharge detection + optional AC sensor to distinguish UPS mode
- Configurable delayed Eco when on battery, automatic Normal on stable mains
- Warn-before-hibernate and explicit opt-in; no auto on stale telemetry
- Per-display policy and DDC/CI capability detection, hotplug

## v0.4 — desktop polish
- Tray, autostart, event history, power profiles, notifications
- Per-app Windows EcoQoS where permitted; avoid forced termination
- Profiles export/import, localization, installer/signing
- Measured performance and hardware compatibility matrix
