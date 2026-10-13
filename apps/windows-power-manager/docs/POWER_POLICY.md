# Power Policy v0.2 — Windows 11 adjustable power manager

All settings change only one temporary copy of the original Windows power scheme, never the original itself. On clean exit, the original plan is restored when the Zilla clone remains active.

| Profile | CPU performance max | HDD idle |
| --- | --- | --- |
| Maximum | 100% | Never |
| Balanced | 80% | 300 seconds |
| Minimum | 55% | 180 seconds |
| Hard | 25% | 120 seconds |
| Manual slider | 20–100% | 300 seconds |
| AUTO | 35–95% (starts at 55% or current cap) | 300 seconds |

Windows processor minimum state is 5% on Zilla profiles. Both AC and DC values are configured. A KEMOT 230 V UPS looks AC-powered to Windows when it is running from LiFePO4.

All managed profiles disable Windows inactivity timers for system sleep (STANDBYIDLE=0), hibernation (HIBERNATEIDLE=0) and display blanking (VIDEOIDLE=0), both AC/DC. No system sleep/hibernate/shutdown APIs are called. Manual Windows/firmware sleep is not prevented. Restoring the original plan restores its original policy.

Secondary-only DDC/CI VCP 0xD6 off/on is explicit, hardware dependent and may fail to wake without the physical monitor button. The separate all-screens-off button broadcasts Windows WM_SYSCOMMAND SC_MONITORPOWER, without suspending Windows; keyboard/mouse usually wakes the monitors. When necessary it first enables a no-sleep cloned profile.

AUTO is disabled by default. It samples global CPU utilization every 3 s; 3 samples >=85% raise the CPU ceiling by 10 points, 10 samples <=20% lower it by 5, and 4 cooldown samples follow a change. Manual and preset controls disable AUTO. High single-core utilization may feel slow even at low total load, so AUTO is heuristic only.

CPU performance % is NOT watts. Real total 230 V UPS power also includes GPU, motherboard, displays and inverter losses; 100 W cannot be guaranteed without a wall meter or proven UPS readings. NVIDIA GPU metrics are read-only chip readings, not UPS load. No GPU tuning is performed.

Hardware verification: save the active scheme GUID; select profiles and check powercfg /query SCHEME_CURRENT SUB_PROCESSOR PROCTHROTTLEMAX and all three inactivity aliases above (AC/DC zero). Confirm return to original GUID on Restore and clean exit. Test monitor wake on the target PC before relying on screen controls.
