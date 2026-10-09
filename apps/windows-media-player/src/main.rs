#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod dsp;
mod library;
mod playback;
mod queue;
mod roadmap;

use audio::{start_audio_worker, AudioController, Command};
use library::{LibraryScanner, ScanEvent};
use playback::{format_duration, Transport};
use queue::PlayQueue;
use slint::{ComponentHandle, Model, SharedString, VecModel};
use std::{cell::{Cell, RefCell}, error::Error, path::PathBuf, rc::Rc, time::Duration};

slint::include_modules!();

const EQ_VISIBLE_MAP: [usize; 15] = [0,2,4,6,8,10,12,14,16,18,20,22,24,27,30];

fn main() -> Result<(), Box<dyn Error>> {
    // Scanner mode is the SAME executable in a separate process with no GUI or audio.
    // This must be checked before initializing Slint/CPAL.
    let mut args = std::env::args_os();
    let _ = args.next();
    if args.next().as_deref() == Some(std::ffi::OsStr::new("--scan-worker")) {
        let folder = args.next().ok_or("Scanner requires a folder path")?;
        library::worker_entry(&PathBuf::from(folder))?;
        return Ok(());
    }

    let ui = AppWindow::new()?;
    // Plan and engineering backlog share one validated, versioned source.
    // "CODE / UNVERIFIED" is distinct from "VERIFIED".
    let plan = roadmap::bundled().map_err(std::io::Error::other)?;
    ui.set_roadmap_summary(plan.milestone_summary.into());
    ui.set_task_summary(plan.task_summary.into());
    ui.set_next_gate(plan.next_gate.into());
    let milestone_model: Rc<VecModel<SharedString>> = Rc::new(
        VecModel::from(plan.milestones.into_iter().map(SharedString::from).collect::<Vec<_>>())
    );
    let task_model: Rc<VecModel<SharedString>> = Rc::new(
        VecModel::from(plan.focus_tasks.into_iter().map(SharedString::from).collect::<Vec<_>>())
    );
    ui.set_roadmap_items(milestone_model.into());
    ui.set_focus_tasks(task_model.into());
    let AudioController { commands, updates } = start_audio_worker();
    let scanner = Rc::new(LibraryScanner::new());
    let scan_serial = Rc::new(Cell::new(0u64));
    let queue = Rc::new(RefCell::new(PlayQueue::default()));
    let model: Rc<VecModel<SharedString>> = Rc::new(VecModel::default());
    ui.set_library_items(model.clone().into());

    let cmd = commands.clone();
    let q = queue.clone();
    let rows = model.clone();
    ui.on_open_file(move || {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Audio", &["mp3","flac","wav","ogg","m4a","aac","opus"]).pick_file()
        {
            q.borrow_mut().append_selected(path.clone());
            refresh_list(&q, &rows);
            let _ = cmd.send(Command::Open(path));
        }
    });

    let folder_cmd = scanner.clone();
    let serial = scan_serial.clone();
    let q = queue.clone();
    let rows = model.clone();
    let weak = ui.as_weak();
    ui.on_open_folder(move || {
        // Native modal file/folder chooser is explicitly user initiated.
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            q.borrow_mut().clear();
            rows.set_vec(Vec::new());
            let id = folder_cmd.scan(folder);
            serial.set(id);
            if let Some(window) = weak.upgrade() {
                window.set_library_count("Scanning…".into());
                window.set_notice("Scanning runs in a child process".into());
            }
        }
    });

    let cmd = commands.clone();
    ui.on_toggle_play(move || { let _ = cmd.send(Command::TogglePlay); });
    let cmd = commands.clone();
    ui.on_stop_playback(move || { let _ = cmd.send(Command::Stop); });
    let cmd = commands.clone();
    ui.on_seek_to(move |v| { let _ = cmd.send(Command::SeekPercent(v)); });
    let cmd = commands.clone();
    ui.on_set_volume(move |v| { let _ = cmd.send(Command::SetVolume(v)); });

    let q = queue.clone();
    let cmd = commands.clone();
    ui.on_play_library_track(move |index| {
        if index >= 0 {
            if let Some(path) = q.borrow_mut().select(index as usize) {
                let _ = cmd.send(Command::Open(path));
            }
        }
    });
    let q = queue.clone();
    let cmd = commands.clone();
    ui.on_next_track(move || {
        if let Some(path) = q.borrow_mut().next() { let _ = cmd.send(Command::Open(path)); }
    });
    let q = queue.clone();
    let cmd = commands.clone();
    ui.on_previous_track(move || {
        if let Some(path) = q.borrow_mut().previous() { let _ = cmd.send(Command::Open(path)); }
    });

    let cmd = commands.clone();
    ui.on_set_eq_band(move |index, db| {
        if index >= 0 {
            if let Some(real_band) = EQ_VISIBLE_MAP.get(index as usize) {
                let _ = cmd.send(Command::SetEqBand(*real_band,db));
            }
        }
    });
    let cmd = commands.clone();
    ui.on_set_preamp(move |db| { let _ = cmd.send(Command::SetPreamp(db)); });
    let cmd = commands.clone();
    ui.on_set_bass(move |db| { let _ = cmd.send(Command::SetBass(db)); });
    let cmd = commands.clone();
    ui.on_set_treble(move |db| { let _ = cmd.send(Command::SetTreble(db)); });
    let cmd = commands.clone();
    ui.on_set_loudness(move |on| { let _ = cmd.send(Command::SetLoudness(on)); });
    let cmd = commands.clone();
    ui.on_enable_eq(move |on| { let _ = cmd.send(Command::EnableEq(on)); });

    // UI only: consume ready messages without blocking. No scanning or decoding.
    let scanner_shutdown = scanner.clone();
    let command_shutdown = commands.clone();
    let weak = ui.as_weak();
    let timer = slint::Timer::default();
    let pending_advance = Rc::new(Cell::new(false));
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(200), move || {
        let Some(window) = weak.upgrade() else { return };
        // Bounded channel: at most 64 scan batches buffered at any time.
        for _ in 0..32 {
            let Ok(event) = scanner.updates.try_recv() else { break };
            match event {
                ScanEvent::Batch(id,paths) if id == scan_serial.get() => {
                    queue.borrow_mut().append(paths);
                    refresh_list(&queue,&model);
                    window.set_library_count(format!("{} tracks", queue.borrow().count()).into());
                }
                ScanEvent::Done(id,total) if id == scan_serial.get() => {
                    window.set_library_count(format!("{total} tracks").into());
                    window.set_notice(format!("Library scan complete: {total} audio files").into());
                }
                ScanEvent::Error(id,reason) if id == scan_serial.get() => {
                    window.set_notice(format!("Scanner error: {reason}").into());
                }
                _ => {}
            }
        }

        let mut latest = None;
        while let Ok(snapshot) = updates.try_recv() { latest = Some(snapshot); }
        if let Some(state) = latest {
            window.set_track_title(state.title.clone().into());
            window.set_playing(state.transport == Transport::Playing);
            window.set_has_track(state.has_track);
            window.set_progress_percent(state.progress_percent());
            window.set_elapsed_text(format_duration(state.position).into());
            window.set_duration_text(state.duration.map(format_duration).unwrap_or_else(|| "--:--".into()).into());
            window.set_notice(audio::status_text(&state).into());
            if state.transport == Transport::Playing { pending_advance.set(false); }
            if state.transport == Transport::Stopped && state.detail == "Track finished" && !pending_advance.get() {
                pending_advance.set(true);
                if let Some(path) = queue.borrow_mut().next() { let _ = commands.send(Command::Open(path)); }
            }
        }
    });

    ui.run()?;
    scanner_shutdown.shutdown();
    let _ = command_shutdown.send(Command::Shutdown);
    Ok(())
}

fn refresh_list(queue: &Rc<RefCell<PlayQueue>>, rows: &Rc<VecModel<SharedString>>) {
    let preview = queue.borrow().preview(10);
    rows.set_vec(preview.into_iter().map(SharedString::from).collect());
}
