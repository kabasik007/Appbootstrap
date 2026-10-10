#!/usr/bin/env python3
"""Validate the Slint 31-band equalizer contract against Rust DSP frequencies.

No third-party packages, network, or Rust toolchain are needed for this check.
Run: python tools/check_ui_contract.py
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DSP = (ROOT / "src" / "dsp.rs").read_text(encoding="utf-8")
UI = (ROOT / "ui" / "app-window.slint").read_text(encoding="utf-8")
APP = (ROOT / "src" / "main.rs").read_text(encoding="utf-8")

band_rust = re.search(
    r"pub const BAND_HZ:\s*\[f32;\s*(\d+)\]\s*=\s*\[([^\]]+)\]",
    DSP, flags=re.S,
)
if band_rust is None:
    raise SystemExit("Missing DSP BAND_HZ array")
size = int(band_rust.group(1))
frequencies = [
    float(token) for token in band_rust.group(2).replace("\n", "").split(",")
    if token.strip()
]
if size != 31 or len(frequencies) != 31:
    raise SystemExit("DSP band array is not exactly 31 frequencies")

band_slint = re.search(
    r"for hz\[i\] in \[([^\]]+)\]\s*:\s*VerticalLayout", UI,
    flags=re.S,
)
if band_slint is None:
    raise SystemExit("Missing full slider labels in Slint")
labels = re.findall(r'"([^"]+)"', band_slint.group(1))
if len(labels) != len(frequencies):
    raise SystemExit(f"UI has {len(labels)} labels, DSP has {len(frequencies)} bands")


def hz(label: str) -> float:
    return float(label.lower().replace("k", "e3"))


for index, (frequency, label) in enumerate(zip(frequencies, labels)):
    display_hz = hz(label)
    if abs(display_hz - frequency) / frequency > 0.02:
        raise SystemExit(
            f"Band {index}: UI {label} Hz and DSP {frequency} Hz differ >2%"
        )

if "const EQ_BAND_COUNT: usize = 31;" not in APP:
    raise SystemExit("Rust UI mapping doesn't allow all 31 bands")
if "Command::SetEqBand(index as usize, db)" not in APP:
    raise SystemExit("Slint-to-DSP direct band mapping is absent")
if 'changed(v) => { root.set-eq-band(i,v); }' not in UI:
    raise SystemExit("Slint sliders are not wired to DSP callback")
if "GRAPHIC EQ / 31 BANDS" not in UI or "ЕКВАЛАЙЗЕР / 31 СМУГА" not in UI:
    raise SystemExit("UA/EN UI headings must reflect all 31 bands")

print("OK: 31 Slint sliders match DSP BAND_HZ, and each is connected to Rust")
