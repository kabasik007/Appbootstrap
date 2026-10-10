# Zilla Power Manager - engineering contract

Read root AGENTS.md plus docs/PROJECT_BRIEF.md and docs/ARCHITECTURE.md before touching code.

- Application profile: desktop-native. Windows 10/11 x64, Rust + Slint.
- Always preserve Windows' current power plan; only mutate cloned schemes.
- Never change battery charging, JK BMS safety settings, BIOS, GPU voltages or process priority without an explicit feature decision and reversible mechanism.
- Never shut down the main monitor in a per-display operation. DDC/CI secondary monitor sleep is opt-in and best effort.
- Display operations and powercfg commands must remain off the UI thread.
- Do not equate CPU utilization or GPU package telemetry with total UPS watts.
- Avoid GPU overclock/undervolt and forced process termination. Expose conservative controls only.
- Include tests and documentation for each new power policy or telemetry adapter.
- Never claim Windows builds, hardware behaviors or tests succeeded without evidence.
- Before releases: format, lint, tests, Windows packaging, monitor restore and power plan rollback checks.
