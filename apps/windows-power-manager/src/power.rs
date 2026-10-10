//! Apply changes only to clones of the current scheme; never edit the user's plan.
//! A crash may leave a clone active. See README for the recovery procedure.
#[cfg(windows)]
use regex::Regex;
#[cfg(windows)]
use std::{collections::HashMap, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode { Eco, Emergency }
impl Mode {
    #[cfg(windows)]
    fn cpu_max(self) -> &'static str {
        match self { Self::Eco => "65", Self::Emergency => "40" }
    }
    #[cfg(windows)]
    fn display_idle(self) -> &'static str {
        match self { Self::Eco => "120", Self::Emergency => "60" }
    }
    #[cfg(windows)]
    fn disk_idle(self) -> &'static str {
        match self { Self::Eco => "120", Self::Emergency => "60" }
    }
    #[cfg(windows)]
    fn name(self) -> &'static str {
        match self { Self::Eco => "Zilla PM Eco", Self::Emergency => "Zilla PM Emergency" }
    }
}
#[derive(Default)]
pub struct PowerPlans {
    #[cfg(windows)]
    original: Option<String>,
    #[cfg(windows)]
    clones: HashMap<Mode, String>,
}
#[cfg(windows)]
fn command(args: &[&str]) -> Result<String, String> {
    let output = Command::new("powercfg").args(args).output()
        .map_err(|e| format!("powercfg не запущено: {e}"))?;
    if !output.status.success() {
        return Err(format!("powercfg {} повернув помилку {}", args.join(" "), output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
#[cfg(windows)]
fn guid(source: &str) -> Result<String, String> {
    let re = Regex::new(r"(?i)[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}")
        .map_err(|e| e.to_string())?;
    re.find(source).map(|m| m.as_str().to_lowercase())
        .ok_or_else(|| "GUID схеми не знайдено".to_owned())
}
#[cfg(windows)]
fn active_guid() -> Result<String, String> { guid(&command(&["/getactivescheme"])?) }

impl PowerPlans {
    pub fn apply(&mut self, mode: Mode) -> Result<String, String> {
        #[cfg(not(windows))]
        { let _ = mode; Err("Потрібен Windows".to_owned()) }
        #[cfg(windows)]
        {
            if self.original.is_none() { self.original = Some(active_guid()?); }
            let base = self.original.as_ref().expect("initialized");
            let id = if let Some(id) = self.clones.get(&mode) {
                id.clone()
            } else {
                let id = guid(&command(&["/duplicatescheme", base])?)?;
                let changes: [[&str; 4]; 6] = [
                    ["SUB_PROCESSOR", "PROCTHROTTLEMAX", mode.cpu_max(), "AC"],
                    ["SUB_PROCESSOR", "PROCTHROTTLEMAX", mode.cpu_max(), "DC"],
                    ["SUB_VIDEO", "VIDEOIDLE", mode.display_idle(), "AC"],
                    ["SUB_VIDEO", "VIDEOIDLE", mode.display_idle(), "DC"],
                    ["SUB_DISK", "DISKIDLE", mode.disk_idle(), "AC"],
                    ["SUB_DISK", "DISKIDLE", mode.disk_idle(), "DC"]
                ];
                let configuration = (|| -> Result<(), String> {
                    command(&["/changename", &id, mode.name()])?;
                    for [sub, setting, value, supply] in changes {
                        let verb = if supply == "AC" { "/setacvalueindex" } else { "/setdcvalueindex" };
                        command(&[verb, &id, sub, setting, value])?;
                    }
                    Ok(())
                })();
                if let Err(e) = configuration {
                    let _ = command(&["/delete", &id]);
                    return Err(e);
                }
                self.clones.insert(mode, id.clone());
                id
            };
            command(&["/setactive", &id])?;
            let name = match mode {
                Mode::Eco => "Eco: CPU до 65%, екрани/HDD у простої через 120 c",
                Mode::Emergency => "Emergency: CPU до 40%, екрани/HDD через 60 c"
            };
            Ok(name.to_owned())
        }
    }
    pub fn normal(&mut self) -> Result<String, String> {
        #[cfg(not(windows))]
        { Ok("No-op on non-Windows".to_owned()) }
        #[cfg(windows)]
        {
            if let Some(original) = &self.original {
                command(&["/setactive", original])?;
                Ok("Відновлено початковий план Windows".to_owned())
            } else {
                Ok("Збережено поточний план Windows".to_owned())
            }
        }
    }
}
impl Drop for PowerPlans {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            if self.original.is_none() { return; }
            let current = active_guid().ok();
            if current.as_ref().is_some_and(|id| self.clones.values().any(|x| x == id)) {
                let _ = self.normal();
            }
            let current = active_guid().ok();
            for id in self.clones.values() {
                if current.as_ref() != Some(id) {
                    let _ = command(&["/delete", id]);
                }
            }
        }
    }
}
