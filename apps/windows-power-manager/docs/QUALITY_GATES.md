# Quality gates

V0.1 CI on windows-latest:
- cargo fmt --all -- --check
- cargo clippy --all-targets -- -D warnings
- cargo test
- cargo build --release
- Upload unsigned exe only if earlier commands succeed.

Manual checks required before calling this release stable:
- Verify Normal restores original power scheme and cleans inactive Zilla clones.
- Verify under 200–600 W load the PC remains stable on real KEMOT UPS transfer.
- With 2+ monitors verify primary never receives a DDC/CI power-off command.
- Test DDC/CI wake/recovery for each monitor brand and HDMI/DP cable.
- Measure idle app memory/CPU, launch latency, and impact on HDDs.
- Test unplug/replug and user profile permission errors; do not promise success without these runs.

Status: implementation and hardware tests not yet verified. GitHub Actions result is authoritative for the Windows build.
