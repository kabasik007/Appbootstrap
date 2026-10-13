//! Windows 11 display topology, not DDC/CI physical monitor power.
//! Unlike DDC/CI, DisplaySwitch removes inactive displays from the desktop
//! so windows do not remain on an invisible-but-logically-connected screen.
//! The main application enforces a timed preview and automatically restores
//! extended mode unless the user confirms.
#[cfg(windows)]
mod platform {
    use std::{
        mem::size_of,
        os::windows::process::CommandExt,
        process::Command,
        ptr,
    };

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DISPLAY_DEVICE_ATTACHED_TO_DESKTOP: u32 = 0x0000_0001;
    const DISPLAY_DEVICE_PRIMARY_DEVICE: u32 = 0x0000_0004;
    const SM_CMONITORS: i32 = 80;

    #[repr(C)]
    struct DisplayDeviceW {
        cb: u32,
        device_name: [u16; 32],
        device_string: [u16; 128],
        state_flags: u32,
        device_id: [u16; 128],
        device_key: [u16; 128],
    }

    impl Default for DisplayDeviceW {
        fn default() -> Self {
            Self {
                cb: size_of::<Self>() as u32,
                device_name: [0; 32],
                device_string: [0; 128],
                state_flags: 0,
                device_id: [0; 128],
                device_key: [0; 128],
            }
        }
    }

    #[link(name = "user32")]
    extern "system" {
        fn EnumDisplayDevicesW(device: *const u16, device_index: u32,
            display_device: *mut DisplayDeviceW, flags: u32) -> i32;
        fn GetSystemMetrics(index: i32) -> i32;
    }

    fn utf16(bytes: &[u16]) -> String {
        let n = bytes.iter().position(|x| *x == 0).unwrap_or(bytes.len());
        String::from_utf16_lossy(&bytes[..n])
    }

    pub fn primary_summary() -> String {
        let count = unsafe { GetSystemMetrics(SM_CMONITORS) }.max(0);
        let mut primary = String::new();
        let mut i: u32 = 0;
        while i < 24 {
            let mut device = DisplayDeviceW::default();
            if unsafe { EnumDisplayDevicesW(ptr::null(), i, &mut device, 0) } == 0 {
                break;
            }
            if device.state_flags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP != 0
                && device.state_flags & DISPLAY_DEVICE_PRIMARY_DEVICE != 0
            {
                primary = format!("{} · {}",
                    utf16(&device.device_name),
                    utf16(&device.device_string));
                break;
            }
            i += 1;
        }
        if primary.is_empty() {
            format!("Windows: {count} активних екранів; головний не визначений")
        } else {
            format!("Windows: {count} активних екранів · головний: {primary}")
        }
    }

    pub fn graphics_adapter_name() -> Option<String> {
        let mut i: u32 = 0;
        let mut fallback = None;
        while i < 24 {
            let mut device = DisplayDeviceW::default();
            if unsafe { EnumDisplayDevicesW(ptr::null(), i, &mut device, 0) } == 0 {
                break;
            }
            let name = utf16(&device.device_string);
            if name.contains("Radeon") || name.contains("NVIDIA") || name.contains("GeForce") {
                return Some(name);
            }
            if fallback.is_none() && device.state_flags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP != 0
                && !name.is_empty() && !name.contains("Microsoft Basic")
            {
                fallback = Some(name);
            }
            i += 1;
        }
        fallback
    }

    // Windows 11 DisplaySwitch supports the numeric PC screen only (1)
    // and Extend (3) modes. These are topology switches, not system sleep.
    // Do not launch DDC/CI physical off commands.
    fn switch(mode: &str) -> Result<(), String> {
        let result = Command::new("DisplaySwitch.exe")
            .arg(mode)
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("DisplaySwitch.exe не запустився: {e}"))?;
        if !result.success() {
            return Err(format!("DisplaySwitch {mode} завершився з помилкою {result}"));
        }
        Ok(())
    }

    pub fn preview_primary() -> Result<String, String> {
        switch("1")?;
        Ok("Попередній перегляд одного екрана. Підтвердь за 20 секунд, інакше повернуться додаткові екрани.".into())
    }

    pub fn restore_extended() -> Result<String, String> {
        switch("3")?;
        Ok("Запитано розширення екранів Windows. Перевір їх порядок і частоти в параметрах дисплея.".into())
    }
}

pub fn primary_summary() -> String {
    #[cfg(windows)]
    { platform::primary_summary() }
    #[cfg(not(windows))]
    { "Екрани Windows недоступні".to_owned() }
}

pub fn graphics_adapter_name() -> Option<String> {
    #[cfg(windows)]
    { platform::graphics_adapter_name() }
    #[cfg(not(windows))]
    { None }
}

pub fn preview_primary() -> Result<String, String> {
    #[cfg(windows)]
    { platform::preview_primary() }
    #[cfg(not(windows))]
    { Err("Потрібна Windows 11".to_owned()) }
}

pub fn restore_extended() -> Result<String, String> {
    #[cfg(windows)]
    { platform::restore_extended() }
    #[cfg(not(windows))]
    { Err("Потрібна Windows 11".to_owned()) }
}
