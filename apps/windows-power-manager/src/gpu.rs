//! Optional read-only NVIDIA telemetry. GPU device watts are NOT AC UPS watts.
#[cfg(windows)]
use std::{path::Path, process::Command};

#[cfg(windows)]
pub fn summary() -> String {
    let fallback = "GPU: н/д (для NVIDIA потрібен nvidia-smi)";
    let executable = if Path::new(
        r"C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe",
    )
    .exists()
    {
        r"C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe"
    } else {
        "nvidia-smi"
    };
    let output = match Command::new(executable)
        .args([
            "--query-gpu=name,utilization.gpu,power.draw",
            "--format=csv,noheader,nounits",
        ])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return fallback.to_owned(),
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(line) = text.lines().next() else {
        return fallback.to_owned();
    };
    let mut columns = line.splitn(3, ',').map(str::trim);
    let name = columns.next().unwrap_or_default();
    let usage = columns.next().unwrap_or_default();
    let watts = columns.next().unwrap_or_default();
    if name.is_empty() {
        return fallback.to_owned();
    }
    let power = if let Ok(watt_value) = watts.parse::<f64>() {
        if watt_value.is_finite() && watt_value >= 0.0 {
            format!("{watt_value:.0} Вт GPU")
        } else {
            "Вт: н/д".to_owned()
        }
    } else {
        "Вт: н/д".to_owned()
    };
    format!("GPU: {name} · {usage}% · {power}")
}

#[cfg(not(windows))]
pub fn summary() -> String {
    "GPU: вимірювання лише у Windows".to_owned()
}
