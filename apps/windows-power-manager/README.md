# Zilla Power Manager — Windows 11 / Rust + Slint

Native desktop power manager with reversible Windows power scheme changes. Intended for use with a 230V KEMOT inverter and 4S LiFePO4 external battery, which Windows cannot detect as an ordinary laptop battery.

## Power controls
- Maximum 100%, Balanced 80%, Minimum 55% and Hard 25% CPU ceiling presets.
- Slider: CPU max processor state 20–100%, changes applied when slider is released.
- AUTO, off by default: sustained high CPU usage raises CPU limit by 10%, sustained low usage reduces it by 5%, with a cooldown. AUTO bounds are 35–95%. Manual controls override it.
- Explicit only-main-screen (DDC/CI secondary monitors off) and restore secondary button.
- Explicit all-screens-off Windows display command, not PC sleep. Keyboard/mouse typically wakes displays.
- Live CPU/RAM measurements and optional NVIDIA GPU chip watts / usage read-only when nvidia-smi is available.
- Manual LiFePO4 SOC and manual UPS watt input for runtime estimates. These are NOT automatic readings.
- Restore original Windows power plan; clean app exit also reverts if the temporary Zilla plan remains active.

## Windows sleep policy
Managed Zilla plans set AC and DC Windows standby, hibernate and display inactivity timeouts to Never. The app never deliberately sleeps or hibernates Windows. Manual OS/firmware sleep and sleep from third-party software remain possible. Choosing Restore original returns whatever Windows sleep settings the user already had.

## Why 100 W is not guaranteed
A CPU performance cap is NOT a watt cap. NVIDIA GPU watts only describe part of the hardware. Total power includes processor, GPU, RAM, motherboard, drives, displays and UPS inverter losses. Accurate 100 W targeting needs a wattmeter or trusted UPS power telemetry. Do not equate a manual calculator input with a measurement.

## Build / verify on Windows 11
Install stable Rust x64 MSVC, Windows SDK, and Microsoft C++ Build Tools. In apps/windows-power-manager run:

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo build --release

Result: target/release/zilla-power-manager.exe. Successful GitHub Actions builds upload an unsigned ZillaPowerManager-Windows-x64 artifact.

## Manual recovery
Record your original active GUID from powercfg /getactivescheme. If the app crashes, powercfg /list shows all plans. Use powercfg /setactive ORIGINAL_GUID to restore, then powercfg /delete INACTIVE_ZILLA_GUID to remove the temporary plan. Never delete an active scheme.

Test both AC and DC profile readback (powercfg /query SCHEME_CURRENT SUB_SLEEP STANDBYIDLE, SUB_SLEEP HIBERNATEIDLE, SUB_VIDEO VIDEOIDLE and SUB_PROCESSOR PROCTHROTTLEMAX). Verify no surprise PC sleep, monitor wake with your specific monitors, and plan restoration on exit.

DDC/CI secondary-monitor sleep may not work for every monitor/cable or may require a physical button to wake. Mechanical HDD standby may increase wear if used frequently. No drivers, BMS writes or GPU voltage modifications are performed.

More details: docs/POWER_POLICY.md and docs/ROADMAP.md.
