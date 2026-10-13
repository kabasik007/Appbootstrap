//! Read-only GPU name through Windows WMI, optional AMD/NVIDIA watt sensors through
//! a running LibreHardwareMonitor instance. Sensor wattage is NOT wall/UPS AC power.
//! No downloads, driver changes, undervolting or hardware writes.
#[cfg(windows)]
use std::{os::windows::process::CommandExt, process::Command};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Default, Debug, Clone)]
struct SensorReading {
    gpu_name: Option<String>,
    gpu_w: Option<f64>,
    gpu_load: Option<f64>,
    cpu_w: Option<f64>,
}

fn parse_readings(output: &str) -> SensorReading {
    let mut result = SensorReading::default();
    for line in output.lines() {
        let Some((key, value)) = line.trim().split_once('=') else { continue };
        let value = value.trim();
        match key {
            "GPU_NAME" if !value.is_empty() => result.gpu_name = Some(value.to_owned()),
            "GPU_W" => result.gpu_w = parse_sensor_value(value, 1000.0),
            "GPU_LOAD" => result.gpu_load = parse_sensor_value(value, 100.0),
            "CPU_W" => result.cpu_w = parse_sensor_value(value, 1000.0),
            _ => {}
        }
    }
    result
}

fn parse_sensor_value(value: &str, max: f64) -> Option<f64> {
    value.parse::<f64>().ok()
        .filter(|x| x.is_finite() && *x >= 0.0 && *x <= max)
}

pub fn summary() -> (String, String) {
    #[cfg(not(windows))]
    { return ("GPU: тільки Windows".to_owned(), "CPU: Вт н/д".to_owned()); }

    #[cfg(windows)]
    {
        let output = Command::new("powershell.exe")
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", POWERSHELL_READONLY])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        let reading = match output {
            Ok(out) if out.status.success() => parse_readings(&String::from_utf8_lossy(&out.stdout)),
            _ => SensorReading::default(),
        };
        // PowerShell/WMI can fail or use an incompatible console encoding.
        // Enumerate installed Windows display adapters natively as a fallback.
        let name = reading.gpu_name
            .or_else(crate::displays::graphics_adapter_name)
            .unwrap_or_else(|| "GPU (модель недоступна)".to_owned());
        let usage = reading.gpu_load.map(|v| format!(" · {:.0}%", v)).unwrap_or_default();
        let gpu_w = reading.gpu_w
            .map(|v| format!(" · {:.1} Вт GPU", v))
            .unwrap_or_else(|| " · Вт: н/д (LibreHardwareMonitor)".to_owned());
        let cpu_w = reading.cpu_w
            .map(|v| format!("CPU package: {:.1} Вт (датчик)", v))
            .unwrap_or_else(|| "CPU package: Вт н/д (LibreHardwareMonitor)".to_owned());
        (format!("{name}{usage}{gpu_w}"), cpu_w)
    }
}

#[cfg(windows)]
const POWERSHELL_READONLY: &str = r#"$ErrorActionPreference = 'SilentlyContinue'
$cards = @(Get-CimInstance -ClassName Win32_VideoController -ErrorAction SilentlyContinue)
$card = $cards | Where-Object { $_.Name -match 'Radeon RX|Radeon Pro|GeForce|RTX|GTX' } | Select-Object -First 1
if (-not $card) { $card = $cards | Where-Object { $_.Name -notmatch 'Microsoft Basic' } | Select-Object -First 1 }
if ($card) { Write-Output ('GPU_NAME=' + $card.Name) }
try {
    $sensors = @(Get-CimInstance -Namespace 'root\LibreHardwareMonitor' -ClassName Sensor -ErrorAction Stop)
    $gpu = @($sensors | Where-Object { $_.Identifier -match '/gpu-' })
    $power = $gpu | Where-Object { $_.SensorType -eq 'Power' } |
        Sort-Object { if ($_.Name -match 'Board|Total') { 0 } elseif ($_.Name -match 'Package') { 1 } else { 2 } } |
        Select-Object -First 1
    $load = $gpu | Where-Object { $_.SensorType -eq 'Load' -and $_.Name -match 'GPU Core|GPU Total|GPU' } |
        Select-Object -First 1
    if ($power -and $null -ne $power.Value) { Write-Output ('GPU_W=' + $power.Value.ToString([Globalization.CultureInfo]::InvariantCulture)) }
    if ($load -and $null -ne $load.Value) { Write-Output ('GPU_LOAD=' + $load.Value.ToString([Globalization.CultureInfo]::InvariantCulture)) }
    $cpu = $sensors | Where-Object { $_.Identifier -match '/cpu|/amdcpu|/intelcpu' -and $_.SensorType -eq 'Power' } |
        Sort-Object { if ($_.Name -match 'Package|CPU Package') { 0 } else { 1 } } |
        Select-Object -First 1
    if ($cpu -and $null -ne $cpu.Value) { Write-Output ('CPU_W=' + $cpu.Value.ToString([Globalization.CultureInfo]::InvariantCulture)) }
} catch {}"#;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_amd_and_cpu_sensor_readings() {
        let r = parse_readings("GPU_NAME=AMD Radeon RX 6900 XT\nGPU_W=45.4\nGPU_LOAD=12.3\nCPU_W=20.8\n");
        assert_eq!(r.gpu_name.as_deref(), Some("AMD Radeon RX 6900 XT"));
        assert_eq!(r.gpu_w, Some(45.4));
        assert_eq!(r.cpu_w, Some(20.8));
        assert_eq!(r.gpu_load, Some(12.3));
    }
    #[test]
    fn rejects_fake_or_out_of_range_metrics() {
        let r = parse_readings("GPU_W=-10\nGPU_LOAD=200\nCPU_W=NaN\n");
        assert!(r.gpu_w.is_none());
        assert!(r.gpu_load.is_none());
        assert!(r.cpu_w.is_none());
    }
}
