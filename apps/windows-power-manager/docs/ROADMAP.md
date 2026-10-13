# Roadmap

## v0.2 — implementation; hardware validation pending
- [x] Four profiles Maximum/Balanced/Minimum/Hard
- [x] Adjustable CPU ceiling 20–100%, applied when slider is released
- [x] Opt-in AUTO with conservative load thresholds and cooldown
- [x] Temporary reversible Windows power plan: never alter original
- [x] No automatic system sleep, hibernate or forced shutdown
- [x] Manual secondary-monitor DDC/CI and all-screens-off commands
- [x] CPU/RAM metrics, optional NVIDIA read-only power/utilization
- [x] Manual LiFePO4 runtime calculation; never pretend watts are measured
- [ ] Verify current commit builds in Windows GitHub Actions
- [ ] Validate actual Windows 11 system, all screens, and plan rollback

## v0.3 — measured power target
- [ ] Read-only JK BMS Bluetooth integration and stale telemetry checks
- [ ] Optional physical wall-wattmeter or verified UPS power telemetry
- [ ] 100 W user target with measured total AC power and bounded adjustments
- [ ] GPU vendor model / ability detection, optional validated power limit with opt-in and rollback
- [ ] Per-monitor DDC/CI compatibility/identification and hotplug

## v0.4 — release polish
- [ ] System tray, startup, profiles persistence and notifications
- [ ] Safe UPS-discharge triggered profile switch from verified sensors
- [ ] GPU/CPU power/performance benchmark on target hardware
- [ ] Signed installer and recovery UX

Never implement forced Windows system sleep or hibernate as a saving strategy.
