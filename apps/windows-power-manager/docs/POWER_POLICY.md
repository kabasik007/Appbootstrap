# Power policy v0.1.1 — no automatic Windows sleep

Normal: restore the original user-selected Windows power scheme without modifying it.

Eco (cloned scheme only): CPU maximum processor state 65%; mechanical HDD idle 120 s; Windows system sleep (standby), hibernation and automatic display blanking timeouts all set to Never (0).

Emergency (cloned scheme only): CPU maximum processor state 40%; mechanical HDD idle 60 s; Windows system sleep (standby), hibernation and automatic display blanking timeouts all set to Never (0).

The cloned plans set both AC and DC values using `powercfg`:
- `SUB_SLEEP/STANDBYIDLE = 0`
- `SUB_SLEEP/HIBERNATEIDLE = 0`
- `SUB_VIDEO/VIDEOIDLE = 0`

The app issues no system sleep/hibernate/shutdown command. Manual sleep by the user, third-party utilities, or firmware-level suspend may still occur. Normal and clean app exit restore the original power plan; crashes/power outages require manual recovery, as described in README. All 230 V KEMOT UPS loads look like AC to Windows, hence both AC/DC changes.

Secondary-monitor off/on is an optional manual DDC/CI VCP 0xD6 operation, NOT PC sleep. The Windows primary display is never targeted. DDC/CI does not necessarily power down all monitors or guarantee remote wake.

HDD standby may still occur after 60–120 s; SSD/NVMe behavior is defined by hardware/firmware. No process kills, GPU voltage modification, USB port disabling, or BMS writes.

To verify on Windows: save original `powercfg /getactivescheme`, select Eco/Emergency, then run:
```powershell
powercfg /query SCHEME_CURRENT SUB_SLEEP STANDBYIDLE
powercfg /query SCHEME_CURRENT SUB_SLEEP HIBERNATEIDLE
powercfg /query SCHEME_CURRENT SUB_VIDEO VIDEOIDLE
```
Verify **both AC and DC** show `0x00000000`. Return to Normal and confirm the initial GUID is active. Hardware validation remains pending.
