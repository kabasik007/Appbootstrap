# Roadmap

## v0.2 — implementation; hardware validation pending
- [x] Four profiles Maximum/Balanced/Minimum/Hard
- [x] Adjustable CPU ceiling 20–100%, applied when slider is released
- [x] Opt-in AUTO with conservative load thresholds and cooldown
- [x] Temporary reversible Windows power plan: never alter original
- [x] No automatic system sleep, hibernate or forced shutdown
- [x] Removed DDC/CI physical monitor-off commands that left invisible logical displays. Added Windows 11 DisplaySwitch (1/3) preview, 20-second rollback, explicit confirmation, and extend recovery; hardware testing pending.
- [x] CPU model/RAM stats, Windows WMI GPU model, optional AMD/NVIDIA & CPU package read-only power data from LibreHardwareMonitor WMI
- [x] Manual LiFePO4 runtime calculation; never pretend watts are measured
- [ ] Verify the latest code revision builds in Windows GitHub Actions
- [ ] Validate actual Windows 11 system, all screens, and plan rollback

## v0.2.1 — Windows 11 UX and hardware check
- [x] Suppress unwanted CMD windows from GUI, powercfg and PowerShell
- [x] Redesigned colored profile tiles, clearer battery and restore sections
- [ ] Validate LibreHardwareMonitor WMI with AMD RX 6900 on user's Windows 11 hardware
- [ ] Compare AMD Adrenalin GPU TBP, component sensors and external UPS input watts
- [ ] Confirm exact Windows primary screen mapping for four monitors, and compare /extend restored positions and refresh rates

## v0.2.3 — informed by read-only Windows 11 diagnostics (implementation in progress)
- [x] Correct CPU topology baseline: 2 × Xeon E5-2678 v3, 24 cores, 48 logical threads.
- [x] AUTO considers busiest logical thread, not only average CPU utilization; add single-thread regression tests.
- [x] Display aggregate CPU usage plus busiest-thread usage.
- [x] Diagnostics reports socket/core/thread totals and RAM modules without serial data.
- [x] Document RX 6900 XT, 3 active Windows displays, DISPLAY2 as primary, and 165 Hz adapter mode.
- [ ] Hardware test AUTO under one busy thread + low total CPU load.
- [ ] Native AMD ADLX read-only TBP adapter with runtime capability probe, no GPU tuning.
- [ ] Compare 165/120/60 Hz with a real total UPS wall-power meter.

## v0.3 — measured power target
- [ ] Read-only JK BMS Bluetooth integration and stale telemetry checks
- [ ] Optional physical wall-wattmeter or verified UPS power telemetry
- [ ] Goal-oriented 200 W / 150 W total AC profile with verified wattmeter and bounded adjustments
- [ ] GPU vendor model / ability detection, optional validated power limit with opt-in and rollback
- [ ] Stable per-display selection using native Windows topology APIs with exact restore snapshot and hotplug handling

## v0.4 — release polish
- [ ] System tray, startup, profiles persistence and notifications
- [ ] Safe UPS-discharge triggered profile switch from verified sensors
- [ ] GPU/CPU power/performance benchmark on target hardware
- [ ] Signed installer and recovery UX

Never implement forced Windows system sleep or hibernate as a saving strategy.
