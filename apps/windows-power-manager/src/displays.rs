//! Best-effort DDC/CI VCP 0xD6 control of *secondary* external monitors.
//! Never sends a power command to the Windows primary monitor.
//! Unsupported display connections are skipped. Users must opt-in with a button.
//! Some monitors cannot wake via DDC/CI once sleeping; use their physical button.
#[cfg(windows)]
mod platform {
    use std::mem::size_of;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect { left: i32, top: i32, right: i32, bottom: i32 }
    #[repr(C)]
    struct MonitorInfo {
        size: u32, monitor: Rect, work: Rect, flags: u32
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct PhysicalMonitor {
        handle: isize,
        description: [u16; 128],
    }
    const MONITORINFOF_PRIMARY: u32 = 1;
    #[link(name = "user32")]
    extern "system" {
        fn EnumDisplayMonitors(hdc: isize, rect: *const Rect,
            callback: Option<unsafe extern "system" fn(isize, isize, *mut Rect, isize) -> i32>,
            data: isize) -> i32;
        fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
    }
    #[link(name = "dxva2")]
    extern "system" {
        fn GetNumberOfPhysicalMonitorsFromHMONITOR(monitor: isize, count: *mut u32) -> i32;
        fn GetPhysicalMonitorsFromHMONITOR(monitor: isize, count: u32,
            physical_monitors: *mut PhysicalMonitor) -> i32;
        fn DestroyPhysicalMonitors(count: u32, physical_monitors: *mut PhysicalMonitor) -> i32;
        fn SetVCPFeature(monitor: isize, code: u8, value: u32) -> i32;
    }
    unsafe extern "system" fn gather(monitor: isize, _hdc: isize,
        _rect: *mut Rect, context: isize) -> i32 {
        let items = &mut *(context as *mut Vec<isize>);
        let mut info = MonitorInfo {
            size: size_of::<MonitorInfo>() as u32,
            monitor: Rect::default(), work: Rect::default(), flags: 0
        };
        if GetMonitorInfoW(monitor, &mut info) != 0
            && (info.flags & MONITORINFOF_PRIMARY) == 0 {
            items.push(monitor);
        }
        1
    }
    pub fn set_power(on: bool) -> Result<String, String> {
        let mut secondary = Vec::<isize>::new();
        let ok = unsafe {
            EnumDisplayMonitors(0, std::ptr::null(), Some(gather),
                (&mut secondary as *mut Vec<isize>) as isize)
        };
        if ok == 0 { return Err("Windows не надала список моніторів".into()); }
        if secondary.is_empty() { return Err("Другорядних моніторів не знайдено".into()); }
        let mut attempted = 0;
        let mut succeeded = 0;
        for monitor in secondary {
            let mut count = 0u32;
            if unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut count) } == 0
                || count == 0 || count > 16 { continue; }
            let empty = PhysicalMonitor { handle: 0, description: [0; 128] };
            let mut physical = vec![empty; count as usize];
            let found = unsafe {
                GetPhysicalMonitorsFromHMONITOR(monitor, count, physical.as_mut_ptr())
            };
            if found == 0 { continue; }
            for target in &physical {
                attempted += 1;
                // VESA DDC/CI power mode (0xD6): 0x01 on, 0x04 off.
                if unsafe { SetVCPFeature(target.handle, 0xD6, if on { 1 } else { 4 }) } != 0 {
                    succeeded += 1;
                }
            }
            unsafe { DestroyPhysicalMonitors(count, physical.as_mut_ptr()); }
        }
        if succeeded == 0 {
            return Err("Жоден додатковий монітор не підтвердив DDC/CI. Перевір DDC/CI в меню дисплея".into());
        }
        Ok(format!("DDC/CI: команду {} прийняли {succeeded}/{attempted} моніторів. Основний не чіпали",
            if on { "відновлення" } else { "сну" }))
    }
}
pub fn set_secondary_power(on: bool) -> Result<String, String> {
    #[cfg(windows)]
    { platform::set_power(on) }
    #[cfg(not(windows))]
    { let _ = on; Err("Керування дисплеями можливе тільки у Windows".into()) }
}
