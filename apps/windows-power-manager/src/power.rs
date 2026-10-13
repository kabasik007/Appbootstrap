//! Reversible Windows power profiles. Never modify the user's original scheme.
//! Automatic sleep/hibernate timeouts are disabled ONLY in our cloned scheme.
//! Manual sleep, firmware power events and other applications remain outside our control.

#[cfg(windows)]
use regex::Regex;
#[cfg(windows)]
use std::process::Command;

pub const MIN_CPU_PERCENT: u8 = 20;
pub const MAX_CPU_PERCENT: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Maximum,
    Balanced,
    Minimum,
    Hard,
}

impl Preset {
    pub fn cpu_cap(self) -> u8 {
        match self {
            Self::Maximum => 100,
            Self::Balanced => 80,
            Self::Minimum => 55,
            Self::Hard => 25,
        }
    }

    pub fn disk_idle_seconds(self) -> u32 {
        match self {
            Self::Maximum => 0,
            Self::Balanced => 300,
            Self::Minimum => 180,
            Self::Hard => 120,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Maximum => "Максимальна",
            Self::Balanced => "Середня",
            Self::Minimum => "Мінімальна",
            Self::Hard => "Hard",
        }
    }
}

pub fn validate_cap(cap: u8) -> Result<(), String> {
    if (MIN_CPU_PERCENT..=MAX_CPU_PERCENT).contains(&cap) {
        Ok(())
    } else {
        Err(format!(
            "Ліміт CPU має бути у діапазоні {MIN_CPU_PERCENT}–{MAX_CPU_PERCENT}%"
        ))
    }
}

#[derive(Default)]
pub struct PowerPlans {
    #[cfg(windows)]
    original: Option<String>,
    #[cfg(windows)]
    clone: Option<String>,
    current_cap: Option<u8>,
}

#[cfg(windows)]
fn command(args: &[&str]) -> Result<String, String> {
    let output = Command::new("powercfg")
        .args(args)
        .output()
        .map_err(|e| format!("Не вдалося запустити powercfg: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "powercfg {} завершився з помилкою {}: {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(windows)]
fn guid(source: &str) -> Result<String, String> {
    let regex = Regex::new(r"(?i)[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}")
        .map_err(|e| e.to_string())?;
    regex
        .find(source)
        .map(|m| m.as_str().to_lowercase())
        .ok_or_else(|| "Не вдалося знайти GUID схеми Windows".to_owned())
}

#[cfg(windows)]
fn active_guid() -> Result<String, String> {
    guid(&command(&["/getactivescheme"])?)
}

#[cfg(windows)]
fn set_index(id: &str, sub: &str, setting: &str, value: &str) -> Result<(), String> {
    for verb in ["/setacvalueindex", "/setdcvalueindex"] {
        command(&[verb, id, sub, setting, value])?;
    }
    Ok(())
}

#[cfg(windows)]
fn configure_scheme(id: &str, cap: u8, disk_idle_seconds: u32) -> Result<(), String> {
    // CPU minimum can scale down at idle. Performance ceiling is NOT a watt cap.
    set_index(id, "SUB_PROCESSOR", "PROCTHROTTLEMIN", "5")?;
    set_index(id, "SUB_PROCESSOR", "PROCTHROTTLEMAX", &cap.to_string())?;
    // Never trigger Windows system sleep/hibernate automatically.
    set_index(id, "SUB_SLEEP", "STANDBYIDLE", "0")?;
    set_index(id, "SUB_SLEEP", "HIBERNATEIDLE", "0")?;
    // Keep monitor sleep as a separate explicit user command.
    set_index(id, "SUB_VIDEO", "VIDEOIDLE", "0")?;
    // Mechanical HDD power-off at short intervals may cause wear.
    set_index(id, "SUB_DISK", "DISKIDLE", &disk_idle_seconds.to_string())?;
    Ok(())
}

impl PowerPlans {
    pub fn apply_preset(&mut self, preset: Preset) -> Result<String, String> {
        self.apply(preset.cpu_cap(), preset.disk_idle_seconds(), preset.label())
    }

    pub fn apply_custom(&mut self, cap: u8) -> Result<String, String> {
        self.apply(cap, 300, "Ручний")
    }

    pub fn apply_adaptive(&mut self, cap: u8) -> Result<String, String> {
        self.apply(cap, 300, "Авто")
    }

    pub fn current_cap(&self) -> Option<u8> {
        self.current_cap
    }

    fn apply(&mut self, cap: u8, disk_idle_seconds: u32, label: &str) -> Result<String, String> {
        validate_cap(cap)?;
        #[cfg(not(windows))]
        {
            let _ = (disk_idle_seconds, label);
            Err("Керування живленням доступне тільки у Windows".to_owned())
        }
        #[cfg(windows)]
        {
            let original = if let Some(original) = &self.original {
                original.clone()
            } else {
                let initial = active_guid()?;
                self.original = Some(initial.clone());
                initial
            };
            let id = if let Some(clone) = &self.clone {
                clone.clone()
            } else {
                let id = guid(&command(&["/duplicatescheme", &original])?)?;
                if let Err(error) = command(&["/changename", &id, "Zilla Power Manager (temporary)"]) {
                    let _ = command(&["/delete", &id]);
                    return Err(error);
                }
                self.clone = Some(id.clone());
                id
            };
            if let Err(error) = configure_scheme(&id, cap, disk_idle_seconds)
                .and_then(|_| command(&["/setactive", &id]).map(|_| ()))
            {
                // If a profile update fails, revert to the original plan.
                let _ = command(&["/setactive", &original]);
                self.current_cap = None;
                return Err(error);
            }
            self.current_cap = Some(cap);
            Ok(format!(
                "{label}: CPU ≤ {cap}% (не ліміт ватів); ПК без автоматичного сну"
            ))
        }
    }

    pub fn ensure_awake_for_display_off(&mut self) -> Result<(), String> {
        if self.current_cap.is_none() {
            // The monitor-off command must not leave the PC using a plan
            // that could automatically suspend on inactivity.
            self.apply(100, 300, "Екрани")?;
        }
        Ok(())
    }

    pub fn normal(&mut self) -> Result<String, String> {
        #[cfg(not(windows))]
        {
            self.current_cap = None;
            Ok("Windows only".to_owned())
        }
        #[cfg(windows)]
        {
            if let Some(original) = &self.original {
                command(&["/setactive", original])?;
            }
            self.current_cap = None;
            Ok("Відновлено початковий план Windows".to_owned())
        }
    }
}

impl Drop for PowerPlans {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            let (Some(original), Some(clone)) = (&self.original, &self.clone) else {
                return;
            };
            // Do not override a power scheme selected by the user after our app.
            if active_guid().ok().as_deref() == Some(clone.as_str()) {
                let _ = command(&["/setactive", original]);
            }
            if active_guid().ok().as_deref() != Some(clone.as_str()) {
                let _ = command(&["/delete", clone]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_ordered_by_cpu_cap() {
        assert_eq!(Preset::Maximum.cpu_cap(), 100);
        assert_eq!(Preset::Balanced.cpu_cap(), 80);
        assert_eq!(Preset::Minimum.cpu_cap(), 55);
        assert_eq!(Preset::Hard.cpu_cap(), 25);
        assert!(Preset::Hard.cpu_cap() >= MIN_CPU_PERCENT);
    }

    #[test]
    fn custom_cpu_limits_are_bounded() {
        assert!(validate_cap(20).is_ok());
        assert!(validate_cap(100).is_ok());
        assert!(validate_cap(19).is_err());
        assert!(validate_cap(101).is_err());
    }

    #[test]
    fn new_manager_does_not_change_system() {
        assert_eq!(PowerPlans::default().current_cap(), None);
    }
}
