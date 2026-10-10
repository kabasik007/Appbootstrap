mod displays;
mod model;
mod power;

#[cfg(windows)]
slint::include_modules!();

#[cfg(windows)]
#[derive(Clone, Copy)]
enum Action { Normal, Eco, Emergency, SecondarySleep, SecondaryWake, Quit }

#[cfg(windows)]
fn estimate(ui: &MainWindow) {
    let soc = ui.get_soc();
    let watts = ui.get_watts();
    ui.set_soc_text(format!("{soc}%").into());
    ui.set_watts_text(format!("{watts} W").into());
    ui.set_runtime_text(model::BatteryModel::default().label(soc as f64, watts as f64).into());
}

#[cfg(windows)]
fn submit(tx: &std::sync::mpsc::Sender<Action>, action: Action, ui: &MainWindow) {
    if tx.send(action).is_err() {
        ui.set_status_text("Не вдалося запустити фонову команду".into());
    } else {
        ui.set_status_text("Застосовую параметри...".into());
    }
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use slint::ComponentHandle;
    use std::sync::mpsc;
    use std::time::Duration;

    let ui = MainWindow::new()?;
    estimate(&ui);
    let weak = ui.as_weak();
    ui.on_settings_changed(move || {
        if let Some(ui) = weak.upgrade() { estimate(&ui); }
    });

    let (tx, rx) = mpsc::channel::<Action>();
    let weak = ui.as_weak();
    let control_thread = std::thread::spawn(move || {
        let mut plans = power::PowerPlans::default();
        while let Ok(action) = rx.recv() {
            if matches!(action, Action::Quit) { break; }
            let result: Result<(String, Option<&'static str>), String> = match action {
                Action::Normal => plans.normal().map(|s| (s, Some("Normal"))),
                Action::Eco => plans.apply(power::Mode::Eco).map(|s| (s, Some("Eco"))),
                Action::Emergency => plans.apply(power::Mode::Emergency).map(|s| (s, Some("Emergency"))),
                Action::SecondarySleep => displays::set_secondary_power(false).map(|s| (s, None)),
                Action::SecondaryWake => displays::set_secondary_power(true).map(|s| (s, None)),
                Action::Quit => unreachable!(),
            };
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    match result {
                        Ok((message, mode)) => {
                            ui.set_status_text(message.into());
                            if let Some(mode) = mode { ui.set_plan_text(mode.into()); }
                        }
                        Err(err) => ui.set_status_text(format!("ПОМИЛКА: {err}").into())
                    }
                }
            });
        }
        // Drop of PowerPlans restores the original plan if our clone is still active.
        // DDC/CI restores secondary monitors on normal exit (best effort).
        let _ = displays::set_secondary_power(true);
    });

    let normal_tx = tx.clone(); let normal_ui = ui.as_weak();
    ui.on_normal_requested(move || {
        if let Some(ui) = normal_ui.upgrade() { submit(&normal_tx, Action::Normal, &ui); }
    });
    let eco_tx = tx.clone(); let eco_ui = ui.as_weak();
    ui.on_eco_requested(move || {
        if let Some(ui) = eco_ui.upgrade() { submit(&eco_tx, Action::Eco, &ui); }
    });
    let emergency_tx = tx.clone(); let emergency_ui = ui.as_weak();
    ui.on_emergency_requested(move || {
        if let Some(ui) = emergency_ui.upgrade() { submit(&emergency_tx, Action::Emergency, &ui); }
    });
    let off_tx = tx.clone(); let off_ui = ui.as_weak();
    ui.on_secondary_sleep_requested(move || {
        if let Some(ui) = off_ui.upgrade() { submit(&off_tx, Action::SecondarySleep, &ui); }
    });
    let on_tx = tx.clone(); let on_ui = ui.as_weak();
    ui.on_secondary_wake_requested(move || {
        if let Some(ui) = on_ui.upgrade() { submit(&on_tx, Action::SecondaryWake, &ui); }
    });

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let weak = ui.as_weak();
    let metrics_thread = std::thread::spawn(move || {
        use sysinfo::System;
        let mut system = System::new();
        system.refresh_cpu_usage();
        while stop_rx.recv_timeout(Duration::from_secs(3)).is_err() {
            system.refresh_cpu_usage();
            system.refresh_memory();
            let cpu = format!("CPU: {:.0}% (не вати)", system.global_cpu_usage());
            let memory = format!("RAM: {:.1}/{:.1} ГБ",
                system.used_memory() as f64 / 1e9, system.total_memory() as f64 / 1e9);
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.set_cpu_text(cpu.into());
                    ui.set_ram_text(memory.into());
                }
            });
        }
    });

    ui.run()?;
    let _ = stop_tx.send(());
    let _ = metrics_thread.join();
    let _ = tx.send(Action::Quit);
    let _ = control_thread.join();
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Zilla Power Manager supports Windows 10/11. Pure calculation tests are portable.");
}
