# Power policy, version 0.1

Normal: restore user-selected Windows power scheme. No CPU cap, disk or display timeout forced by this app.
Eco: CPU maximum processor state 65%, screen inactivity timeout 120 s, HDD standby timeout 120 s.
Emergency: CPU maximum processor state 40%, screen inactivity timeout 60 s, HDD standby timeout 60 s.

Each plan is duplicated from the user's original active plan. Both AC and DC values are set because the PC continues to receive mains-like 230 V from the external UPS on battery.

Secondary monitor buttons: read Windows monitor topology, ignore MONITORINFOF_PRIMARY, send DDC/CI VCP 0xD6=4 to secondary physical monitors and 0xD6=1 on restore. This is an opt-in best-effort hardware feature; keep the primary visible. This does not remove the display from Windows, guarantee savings, or guarantee remote wake.

Mechanical HDDs may repeatedly spin up with aggressive timeouts, causing wear/latency. Timeout defaults at 60–120 s are intentionally moderate; SSD/NVMe standby is OS/firmware-defined.
No forcing process sleep/kill, GPU undervolt, disabling USB ports, Windows Update manipulation or hibernation.

Planned fail-safe: automated battery policy only after read-only telemetry is validated and freshness known, with user opt-in and a clearly visible pause toggle.
