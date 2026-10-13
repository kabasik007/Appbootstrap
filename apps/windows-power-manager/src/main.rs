#![cfg_attr(windows, windows_subsystem = "windows")]

mod adaptive;
mod displays;
mod gpu;
mod model;
mod power;

#[cfg(windows)]
slint::include_modules!();

#[cfg(windows)]
#[derive(Clone, Copy)]
enum Action {
    Preset(power::Preset),
    Manual(u8),
    AutoToggle,
    ObserveCpu(f32, f32),
    PreviewPrimary,
    ConfirmPrimary,
    RestoreExtended,
    RestoreWindows,
    Quit,
}

#[cfg(windows)]
struct Feedback {
    message: String,
    profile: Option<String>,
    cap: Option<u8>,
    automatic: Option<bool>,
}

#[cfg(windows)]
impl Feedback {
    fn message(message: String) -> Self {
        Self {
            message,
            profile: None,
            cap: None,
            automatic: None,
        }
    }

    fn profile(message: String, name: &str, cap: u8, automatic: bool) -> Self {
        Self {
            message,
            profile: Some(name.to_owned()),
            cap: Some(cap),
            automatic: Some(automatic),
        }
    }
}

#[cfg(windows)]
fn estimate(ui: &MainWindow) {
    let soc = ui.get_soc();
    let watts = ui.get_watts();
    ui.set_soc_text(format!("{soc:.0}%").into());
    ui.set_watts_text(format!("{watts:.0} Вт (ручні)").into());
    ui.set_runtime_text(model::BatteryModel::default().label(soc as f64, watts as f64).into());
}

#[cfg(windows)]
fn submit(tx: &std::sync::mpsc::Sender<Action>, action: Action, ui: &MainWindow) {
    if tx.send(action).is_err() {
        ui.set_status_text("Фоновий потік керування недоступний".into());
    } else {
        ui.set_status_text("Застосовую налаштування Windows...".into());
    }
}

#[cfg(windows)]
fn feedback_to_ui(weak: slint::Weak<MainWindow>, feedback: Result<Feedback, String>) {
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = weak.upgrade() {
            match feedback {
                Ok(feedback) => {
                    ui.set_status_text(feedback.message.into());
                    if let Some(label) = feedback.profile {
                        ui.set_plan_text(label.into());
                    }
                    if let Some(cap) = feedback.cap {
                        ui.set_cpu_limit(cap.into());
                        ui.set_limit_text(format!("CPU ≤ {cap}%").into());
                    }
                    ui.set_display_text(displays::primary_summary().into());
                    if let Some(automatic) = feedback.automatic {
                        ui.set_auto_text(if automatic {
                            "АВТО: УВІМКНЕНО".into()
                        } else {
                            "АВТО: ВИМКНЕНО".into()
                        });
                    }
                }
                Err(message) => ui.set_status_text(format!("ПОМИЛКА: {message}").into()),
            }
        }
    });
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use slint::ComponentHandle;
    use std::{sync::mpsc, time::{Duration, Instant}};

    let ui = MainWindow::new()?;
    estimate(&ui);
    ui.set_display_text(displays::primary_summary().into());
    let weak = ui.as_weak();
    ui.on_settings_changed(move || {
        if let Some(ui) = weak.upgrade() {
            estimate(&ui);
        }
    });

    let (tx, rx) = mpsc::channel::<Action>();
    let weak = ui.as_weak();
    let control_thread = std::thread::spawn(move || {
        let mut plans = power::PowerPlans::default();
        let mut auto = adaptive::AdaptiveCpu::default();
        let mut screen_changed = false;
        let mut preview_since: Option<Instant> = None;
        loop {
            let action = if let Some(started) = preview_since {
                match rx.recv_timeout(Duration::from_millis(900)) {
                    Ok(action) => action,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let remaining = 20_u64.saturating_sub(started.elapsed().as_secs());
                        if remaining == 0 {
                            let result = displays::restore_extended();
                            if result.is_ok() {
                                screen_changed = false;
                            }
                            preview_since = None;
                            feedback_to_ui(weak.clone(), result.map(|_| Feedback::message(
                                "20 секунд минуло — відновлено розширені екрани Windows.".to_owned())));
                        } else {
                            feedback_to_ui(weak.clone(), Ok(Feedback::message(format!(
                                "Перевір головний монітор і натисни «ЗАЛИШИТИ». Автовідновлення через {remaining} с."
                            ))));
                        }
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            } else {
                match rx.recv() {
                    Ok(action) => action,
                    Err(_) => break,
                }
            };
            if matches!(action, Action::Quit) {
                break;
            }
            let feedback: Result<Feedback, String> = match action {
                Action::Preset(profile) => {
                    let result = plans.apply_preset(profile);
                    if result.is_ok() {
                        auto.disable();
                    }
                    result.map(|message| {
                        Feedback::profile(message, profile.label(), profile.cpu_cap(), false)
                    })
                }
                Action::Manual(cap) => {
                    let result = plans.apply_custom(cap);
                    if result.is_ok() {
                        auto.disable();
                    }
                    result.map(|message| Feedback::profile(message, "Ручний", cap, false))
                }
                Action::AutoToggle => {
                    if auto.active() {
                        auto.disable();
                        Ok(Feedback {
                            message: "Адаптація зупинена. Поточний ліміт CPU збережений".to_owned(),
                            profile: Some("Ручний".to_owned()),
                            cap: plans.current_cap(),
                            automatic: Some(false),
                        })
                    } else {
                        let cap = plans.current_cap().unwrap_or(55).clamp(35, 95);
                        plans.apply_adaptive(cap).map(|message| {
                            auto.enable(cap);
                            Feedback::profile(
                                format!("{message}. Авто: +10% при навантаженні, -5% у простої"),
                                "Авто",
                                cap,
                                true,
                            )
                        })
                    }
                }
                Action::ObserveCpu(usage, peak_core) => {
                    if let Some(next_cap) = auto.observe_with_peak(usage, peak_core) {
                        let result = plans.apply_adaptive(next_cap);
                        match result {
                            Ok(message) => Ok(Feedback::profile(message, "Авто", next_cap, true)),
                            Err(message) => {
                                auto.disable();
                                Ok(Feedback {
                                    message: format!("ПОМИЛКА: {message}; авто вимкнено"),
                                    profile: Some("Профіль недоступний".to_owned()),
                                    cap: None,
                                    automatic: Some(false),
                                })
                            }
                        }
                    } else {
                        continue;
                    }
                }
                Action::PreviewPrimary => {
                    let result = displays::preview_primary();
                    if result.is_ok() {
                        screen_changed = true;
                        preview_since = Some(Instant::now());
                    }
                    result.map(Feedback::message)
                }
                Action::ConfirmPrimary => {
                    if preview_since.take().is_some() {
                        Ok(Feedback::message(
                            "Один монітор підтверджено. Натисни «РОЗШИРИТИ», щоб повернути 4 екрани. Вихід із Zilla також відновить розширення.".into()
                        ))
                    } else {
                        Ok(Feedback::message("Немає активного попереднього перегляду.".into()))
                    }
                }
                Action::RestoreExtended => {
                    let result = displays::restore_extended();
                    if result.is_ok() {
                        screen_changed = false;
                        preview_since = None;
                    }
                    result.map(Feedback::message)
                }
                Action::RestoreWindows => {
                    let result = plans.normal();
                    if screen_changed && displays::restore_extended().is_ok() {
                        screen_changed = false;
                        preview_since = None;
                    }
                    if result.is_ok() {
                        auto.disable();
                    }
                    result.map(|message| Feedback {
                        message,
                        profile: Some("Windows (початковий)".to_owned()),
                        cap: None,
                        automatic: Some(false),
                    })
                }
                Action::Quit => unreachable!(),
            };
            feedback_to_ui(weak.clone(), feedback);
        }
        // Clean exit: restore topology whenever Zilla changed it.
        // DisplaySwitch /extend is best-effort and may not reproduce
        // all custom 4-monitor placements; Windows Display settings can.
        if screen_changed {
            let _ = displays::restore_extended();
        }
        // PowerPlans Drop re-activates original plan only if our clone is active.
    });

    let presets = tx.clone();
    let weak = ui.as_weak();
    ui.on_preset_requested(move |id| {
        let preset = match id {
            0 => power::Preset::Maximum,
            1 => power::Preset::Balanced,
            2 => power::Preset::Minimum,
            3 => power::Preset::Hard,
            _ => return,
        };
        if let Some(ui) = weak.upgrade() {
            submit(&presets, Action::Preset(preset), &ui);
        }
    });

    let manual_tx = tx.clone();
    let weak = ui.as_weak();
    ui.on_manual_requested(move |value| {
        if let Some(ui) = weak.upgrade() {
            let cap = value.round().clamp(
                f32::from(power::MIN_CPU_PERCENT),
                f32::from(power::MAX_CPU_PERCENT),
            ) as u8;
            submit(&manual_tx, Action::Manual(cap), &ui);
        }
    });

    let auto_tx = tx.clone();
    let weak = ui.as_weak();
    ui.on_auto_requested(move || {
        if let Some(ui) = weak.upgrade() {
            submit(&auto_tx, Action::AutoToggle, &ui);
        }
    });

    let preview_tx = tx.clone();
    let weak = ui.as_weak();
    ui.on_preview_primary_requested(move || {
        if let Some(ui) = weak.upgrade() {
            submit(&preview_tx, Action::PreviewPrimary, &ui);
        }
    });

    let confirm_tx = tx.clone();
    let weak = ui.as_weak();
    ui.on_confirm_primary_requested(move || {
        if let Some(ui) = weak.upgrade() {
            submit(&confirm_tx, Action::ConfirmPrimary, &ui);
        }
    });

    let restore_display_tx = tx.clone();
    let weak = ui.as_weak();
    ui.on_restore_extended_requested(move || {
        if let Some(ui) = weak.upgrade() {
            submit(&restore_display_tx, Action::RestoreExtended, &ui);
        }
    });

    let restore_tx = tx.clone();
    let weak = ui.as_weak();
    ui.on_restore_requested(move || {
        if let Some(ui) = weak.upgrade() {
            submit(&restore_tx, Action::RestoreWindows, &ui);
        }
    });

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let weak = ui.as_weak();
    let cpu_tx = tx.clone();
    let metrics_thread = std::thread::spawn(move || {
        use sysinfo::System;
        let mut system = System::new();
        system.refresh_cpu_usage();
        let mut count: u64 = 0;
        loop {
            if stop_rx.recv_timeout(Duration::from_secs(3)).is_ok() {
                break;
            }
            system.refresh_cpu_usage();
            system.refresh_memory();
            let utilization = system.global_cpu_usage();
            let peak_core = system.cpus().iter()
                .map(|cpu| cpu.cpu_usage())
                .filter(|value| value.is_finite())
                .fold(0.0_f32, f32::max);
            let _ = cpu_tx.send(Action::ObserveCpu(utilization, peak_core));
            let cpu_name = system.cpus().first()
                .map(|cpu| cpu.brand().trim())
                .filter(|name| !name.is_empty())
                .unwrap_or("CPU: модель недоступна");
            let cpu_text = format!("{cpu_name} · {utilization:.0}% загалом · до {peak_core:.0}% на потоці");
            let ram_text = format!(
                "RAM: {:.1}/{:.1} ГБ",
                system.used_memory() as f64 / 1e9,
                system.total_memory() as f64 / 1e9
            );
            // Read AMD/NVIDIA model and optional LibreHardwareMonitor GPU/CPU sensors every ~12s.
            let gpu_text = if count.is_multiple_of(4) {
                Some(gpu::summary())
            } else {
                None
            };
            count += 1;
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.set_cpu_text(cpu_text.into());
                    ui.set_ram_text(ram_text.into());
                    if let Some((gpu_text, cpu_watts)) = gpu_text {
                        ui.set_gpu_text(gpu_text.into());
                        ui.set_cpu_watts_text(cpu_watts.into());
                    }
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
    eprintln!("Zilla Power Manager supports Windows 10/11. Model tests are portable.");
}
