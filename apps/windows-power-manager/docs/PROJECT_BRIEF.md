# Zilla Power Manager — Project brief

- Owner: kabasik007
- Product: native Windows energy-saving and UPS companion
- Profile: desktop-native
- Target: Windows 10/11, x64; intended PC can demand approximately 200 to 600 W, KEMOT PROsinus 2000-LFP4 12 V, JK BMS attached to 4S LiFePO4 configured 334 Ah
- Distribution: unsigned zip/exe from private use or GitHub Actions artifacts, installer later
- Version: 0.1 development
- License: TBD (the parent template has no selected license)

## Goals

1. Keep the user's primary monitor available while optional secondary displays enter low-power mode.
2. Make CPU and disk idle behavior configurable via reversible Windows native power schemes.
3. Provide transparent battery runtime estimates, without presenting guessed watts as measured data.
4. Prepare a read-only JK BMS transport to enable safe automatic UPS-aware behavior later.

## v0.1 acceptance criteria

- Normal/Eco/Emergency commands operate on independent cloned Windows schemes, preserving the original.
- On clean exit, the original scheme is restored when a Zilla clone is currently active.
- Primary display is never passed to DDC/CI power-off functions.
- Clear manual recovery notes exist for crash/power loss and monitors that cannot wake over DDC/CI.
- Formula tests cover reserve, invalid inputs and changes in load.
- No BMS writes, forced process shutdown, emergency auto-hibernation or stealth service.

## Explicit non-goals in v0.1

- No guaranteed real-watt metering or calibrated runtime.
- No exact power limit for CPU/GPU, no aggressive SSD shutdown, no disabling system devices.
- No automatic state transitions based on missing UPS communication.
- No silent changes to JK BMS, BIOS or monitor configurations.

## Open questions

- Exact JK BMS hardware revision and BLE version.
- GPU vendor and watt metering capability; monitor brands/links and DDC/CI support.
- Preferred hibernation threshold and permission model.
- Source of reliable UPS battery/AC state (read-only BMS BLE, ESP bridge or USB AC sensor).
