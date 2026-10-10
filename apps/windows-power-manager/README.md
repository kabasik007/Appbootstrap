# Zilla Power Manager

Native Windows 10/11 UPS companion written in Rust + Slint, following the desktop-native profile and engineering rules from Appbootstrap.

## What v0.1 does

- Three manual modes: Normal, Eco (processor performance ceiling 65%, idle screen/HDD 120 seconds), Emergency (40%, 60 seconds).
- Changes only duplicates of the original Windows power plan; Normal and clean exit restore the previous plan.
- Manual secondary monitor sleep / wake via VESA DDC/CI (VCP 0xD6). Windows' primary monitor is never targeted.
- Displays CPU utilization and RAM usage; runs sampling off the UI thread.
- Estimates runtime for a *configured* 4S LiFePO4 334 Ah battery and a KEMOT 12 V inverter. SOC and load are MANUAL; no fake metering.
- All functions offline. No process kills, registry tuning, driver install or automatic power events yet.

## Build and test

Install Rust stable for Windows x64 MSVC, Microsoft C++ Build Tools + Windows SDK (IDE not necessary).

From repository root:
    cd apps/windows-power-manager
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo build --release

Build artifacts: target/release/zilla-power-manager.exe. The Windows GitHub Actions workflow in this branch produces an unsigned build artifact on successful compile.

## Risks and limitations

- Desktop computers running on an external 230 V KEMOT UPS appear AC-powered to Windows even when KEMOT is running from the battery. Automatic UPS transfer detection is NOT implemented.
- 334 Ah is the JK BMS configured capacity; cell model not independently confirmed. Efficiency (85%) and inverter standby consumption (15 W) are hypothetical; use a wattmeter to calibrate.
- Windows power plan CPU percentages are *performance ceilings*, not exact watt limits. Platform firmware may not honor every setting.
- Windows HDD idle settings do not immediately spin down every disk; SSD/NVMe drives normally do not benefit from mechanical disk standby.
- DDC/CI requires compatible external monitors and a cable/adapter passing DDC commands. It can be disabled in the monitor OSD. Sending DDC/CI power-off can leave a monitor requiring its physical button to wake. Leave the primary display active and test the secondary displays individually with access to their controls.
- Per-monitor power commands do not guarantee zero power draw or remove a display from Windows layout.
- Sudden crash/power outage may leave a cloned Windows plan active. Recovery: open an elevated terminal; run powercfg /list; activate your old plan with powercfg /setactive GUID; delete inactive Zilla clones with powercfg /delete GUID. Never delete an active Windows plan.
- Forced shutdown cannot guarantee DDC/CI recovery. Use the monitor hardware power button if necessary.
- JK BMS Bluetooth, actual wall-watt measurements, automatic battery detection and hibernation are NOT in v0.1.

## Safe test sequence

1. Save work. Record the active scheme from powercfg /getactivescheme.
2. Launch Normal first and set manual SOC/watts; inspect the calculator.
3. Try Eco and verify the new Zilla plan from powercfg /list; confirm your applications work as expected.
4. Switch back to Normal, confirm the original GUID.
5. Enable DDC/CI on one secondary monitor, then test the secondary sleep button. Ensure primary stays visible, and test wake button.
6. If monitor wake fails, press its physical power button. Do not rely on it for emergency shutdown.
7. Run Windows tests on actual PC/monitor hardware before adopting the app as a production UPS safety mechanism.

Project contracts: docs/PROJECT_BRIEF.md, docs/ARCHITECTURE.md, docs/QUALITY_GATES.md, docs/ROADMAP.md.
