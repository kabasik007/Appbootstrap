# Zilla Power Manager — Windows 11 / Rust + Slint

Native desktop power manager with reversible Windows power scheme changes. Intended for use with a 230V KEMOT inverter and 4S LiFePO4 external battery, which Windows cannot detect as an ordinary laptop battery.

## Power controls
- Maximum 100%, Balanced 80%, Minimum 55% and Hard 25% CPU ceiling presets, now displayed as colored profile tiles.
- Slider: CPU max processor state 20–100%, changes applied when slider is released.
- AUTO, off by default: uses both global CPU usage and the busiest logical CPU. Sustained >85% utilization of **any** logical core (even with ~2% global load on this 48-thread dual Xeon system) raises the cap by 10%. It only lowers the cap after sustained low global load and low peak-core load, with a cooldown. AUTO bounds are 35–95%. Manual controls override it.
- Safe 4-monitor topology switch: **One screen (20-second preview)**, **Keep**, **Restore extended** using Windows 11 DisplaySwitch numeric modes 1/3. Does not send DDC/CI physical-off commands. Timed preview automatically tries to restore extended mode unless confirmed; normal exit also restores extended mode if Zilla changed it.
- No all-screens-off control: it was intentionally removed to avoid leaving the user without a visible monitor. Only reversible Windows topology controls remain.
- CPU model and live load, RAM usage, and read-only GPU model through Windows WMI. Optional AMD/NVIDIA GPU power/utilization and CPU package power are read from **LibreHardwareMonitor** sensors when that application is running and exposes WMI. Without these sensors, watt fields display `н/д` (not guessed).
- Manual LiFePO4 SOC and manual UPS watt input for runtime estimates. These are NOT automatic readings.
- Restore original Windows power plan; clean app exit also reverts if the temporary Zilla plan remains active.

## No black CMD window

The Windows EXE uses the Windows GUI subsystem. Invoked `powercfg` and read-only hardware telemetry PowerShell processes set `CREATE_NO_WINDOW` so they do not flash console windows. In case of a startup error, run the earlier build or inspect GitHub Actions; GUI builds do not print to a visible console.

## AMD RX 6900 XT power telemetry

1. Open **AMD Software: Adrenalin Edition → Performance → Metrics**, and enable GPU Total Board Power (TBP) to compare values.
2. Optional for reading watts inside Zilla: start LibreHardwareMonitor from its **official** GitHub releases page `https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/releases`. Some sensors need administrator rights. Keep LibreHardwareMonitor open while testing.
3. Zilla queries `root\\LibreHardwareMonitor` WMI sensors periodically, read-only. **Some LibreHardwareMonitor releases (including reports for 0.9.5) have broken WMI output**; if watts are N/A, use the included diagnostic script and check sensor availability. No driver installation or AMD power-limit changes. Component watts are not 230 V UPS consumption.
4. Without WMI, Zilla also attempts to read the Radeon name natively through Win32 `EnumDisplayDevicesW`.

## Diagnostics (read-only)

Download **Raw** `scripts/diagnose_windows11.ps1`, saved as **UTF-8 with BOM** for compatibility with Windows PowerShell 5.1. The CI uses Windows PowerShell 5.1 to verify the BOM, parse syntax, and actually produce a report.

For a script saved to H:\\, run:

    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "H:\\diagnose_windows11.ps1"

If an older locally saved script shows mojibake such as `Р”Р°С‚Р°`, convert its bytes once using PowerShell 5.1:

    $path = 'H:\\diagnose_windows11.ps1'
    $text = [System.IO.File]::ReadAllText($path, [System.Text.Encoding]::UTF8)
    [System.IO.File]::WriteAllText($path, $text, (New-Object System.Text.UTF8Encoding($true)))

A `Set-Location H:\\" ` error shown when opening a PowerShell shell normally comes from the user's PowerShell profile, not this script. The `-NoProfile` argument avoids that startup profile for diagnostics.

The file `Zilla_Diagnostic_Windows11.txt` is created on your Desktop. It lists CPU/GPU identity, Windows primary monitor and active display IDs, powercfg AC/DC values and available power sensors; no sleep or display changes.

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

Windows primary screen is a logical designation which can differ from the physical screen you think of as main. If the Windows primary monitor is physically switched off, one-screen preview may temporarily show no visible image; after 20 seconds the app attempts to restore extended mode. The numeric `DisplaySwitch` arguments are Windows 11-specific; restore extended mode can change custom monitor placement. Check Windows Settings → System → Display and Win+P if the desktop looks wrong. The app does not physically cut monitor power. Mechanical HDD standby may increase wear if used frequently. No drivers, BMS writes or GPU voltage modifications are performed.

For the measured machine (2 Xeon E5-2678 v3, Radeon RX 6900 XT, 3 active Windows displays and one additional physical monitor), see `docs/HARDWARE_BASELINE_WINDOWS11_UA.md`. Real GPU TBP remains unavailable without working ADLX or a sensor provider; no total AC watts are guessed. Diagnostic script also reports physical CPU socket count, DIMM count and Windows display-primary identification.\n\nMore details: docs/POWER_POLICY.md and docs/ROADMAP.md.
