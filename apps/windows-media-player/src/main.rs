// Alpha Windows test build uses console subsystem so --scan-worker can reliably
// inherit JSONL stdout pipes. Split GUI/scanner binaries before public release.

mod audio;
mod decode_worker;
mod dsp;
mod library;
mod playback;
mod pcm_ring;
mod playlist;
mod queue;
mod roadmap;
mod session;
mod visualizer;

use audio::{start_audio_worker, AudioController, Command};
use library::{LibraryScanner, ScanEvent};
use playback::{format_duration, Transport};
use queue::PlayQueue;
use session::{SavedSession, SessionStore};
use slint::{ComponentHandle, Model, SharedString, VecModel};
use visualizer::BANDS;
use std::{cell::{Cell, RefCell}, error::Error, path::PathBuf, rc::Rc,
    sync::{mpsc::{self, Sender}, Arc}, thread, time::Duration};

slint::include_modules!();

enum PlaylistEvent {
    Imported(Result<Vec<PathBuf>, String>),
    Saved(Result<usize, String>),
}

fn import_playlist_job(path: PathBuf, replies: Sender<PlaylistEvent>) {
    let _ = thread::Builder::new()
        .name("zillaplayer-playlist-import".into())
        .spawn(move || {
            let outcome = playlist::read(&path).map_err(|error| error.to_string());
            let _ = replies.send(PlaylistEvent::Imported(outcome));
        });
}

fn save_playlist_job(path: PathBuf, tracks: Vec<PathBuf>, replies: Sender<PlaylistEvent>) {
    let _ = thread::Builder::new()
        .name("zillaplayer-playlist-save".into())
        .spawn(move || {
            let count = tracks.len();
            let outcome = playlist::write(&path, &tracks)
                .map(|()| count).map_err(|error| error.to_string());
            let _ = replies.send(PlaylistEvent::Saved(outcome));
        });
}

const EQ_BAND_COUNT: usize = 31;

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
    let AudioController { commands, updates, spectrum } = start_audio_worker();
    // Spectrum values are published by an isolated FFT thread. Slint receives
    // only throttled ready-to-draw f32 model updates, never PCM arrays.
    let spectrum_model: Rc<VecModel<f32>> =
        Rc::new(VecModel::from(vec![0.0_f32; BANDS]));
    ui.set_spectrum_bars(spectrum_model.clone().into());
    let spectrum_for_ui = Arc::clone(&spectrum);
    let visual_window = ui.as_weak();
    let visual_timer = slint::Timer::default();
    visual_timer.start(slint::TimerMode::Repeated, Duration::from_millis(50), move || {
        let Some(window) = visual_window.upgrade() else { return; };
        // No pointless render-model work while the Library/Downloader is open.
        if window.get_active_page().as_str() != "Player" { return; }
        for (index, level) in spectrum_for_ui.read().iter().enumerate() {
            // Skip identical values to avoid needless layout/render invalidation.
            if spectrum_model.row_data(index).is_none_or(|old| (old - level).abs() > 0.75) {
                spectrum_model.set_row_data(index, *level);
            }
        }
    });
    let scanner = Rc::new(LibraryScanner::new());
    let scan_serial = Rc::new(Cell::new(0u64));
    let queue = Rc::new(RefCell::new(PlayQueue::default()));
    let model: Rc<VecModel<SharedString>> = Rc::new(VecModel::default());
    ui.set_library_items(model.clone().into());
    // Single background session I/O worker. All disk reads/writes stay off UI.
    let session = Rc::new(SessionStore::start()?);
    let session_dirty = Rc::new(Cell::new(false));
    let user_touched_queue = Rc::new(Cell::new(false));
    let (playlist_tx, playlist_rx) = mpsc::channel::<PlaylistEvent>();

    let import_tx = playlist_tx.clone();
    let imported_before_restore = user_touched_queue.clone();
    ui.on_import_playlist(move || {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("M3U playlists", &["m3u", "m3u8"])
            .pick_file()
        {
            imported_before_restore.set(true);
            import_playlist_job(path, import_tx.clone());
        }
    });

    let save_tx = playlist_tx.clone();
    let save_queue = Rc::clone(&queue);
    ui.on_export_playlist(move || {
        if let Some(mut path) = rfd::FileDialog::new()
            .add_filter("M3U8 playlists", &["m3u8"])
            .set_file_name("ZillaPlayer.m3u8")
            .save_file()
        {
            if path.extension().is_none() { path.set_extension("m3u8"); }
            save_playlist_job(path, save_queue.borrow().snapshot(), save_tx.clone());
        }
    });

    let cmd = commands.clone();
    let q = queue.clone();
    let rows = model.clone();
    let touched = user_touched_queue.clone();
    let dirty = session_dirty.clone();
    ui.on_open_file(move || {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Audio", &["mp3","flac","wav","ogg","m4a","aac","opus"]).pick_file()
        {
            touched.set(true);
            dirty.set(true);
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
    let touched = user_touched_queue.clone();
    let dirty = session_dirty.clone();
    ui.on_open_folder(move || {
        // Native modal file/folder chooser is explicitly user initiated.
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            touched.set(true);
            dirty.set(true);
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
    let dirty = session_dirty.clone();
    ui.on_play_library_track(move |index| {
        if index >= 0 {
            if let Some(path) = q.borrow_mut().select(index as usize) {
                dirty.set(true);
                let _ = cmd.send(Command::Open(path));
            }
        }
    });
    let q = queue.clone();
    let cmd = commands.clone();
    let dirty = session_dirty.clone();
    ui.on_next_track(move || {
        if let Some(path) = q.borrow_mut().next() {
            dirty.set(true);
            let _ = cmd.send(Command::Open(path));
        }
    });
    let q = queue.clone();
    let cmd = commands.clone();
    let dirty = session_dirty.clone();
    ui.on_previous_track(move || {
        if let Some(path) = q.borrow_mut().previous() {
            dirty.set(true);
            let _ = cmd.send(Command::Open(path));
        }
    });

    let cmd = commands.clone();
    ui.on_set_eq_band(move |index, db| {
        if index >= 0 && (index as usize) < EQ_BAND_COUNT {
            let _ = cmd.send(Command::SetEqBand(index as usize, db));
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
    let session_for_updates = Rc::clone(&session);
    let dirty_for_updates = Rc::clone(&session_dirty);
    let touched_before_restore = Rc::clone(&user_touched_queue);
    let session_shutdown_queue = Rc::clone(&queue);
    let weak = ui.as_weak();
    let timer = slint::Timer::default();
    let pending_advance = Rc::new(Cell::new(false));
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(200), move || {
        let Some(window) = weak.upgrade() else { return };
        // Restore only if user has not already started a scan or changed the queue.
        // Loading happens in the persistence worker; nothing reads disk here.
        if let Some(loaded) = session_for_updates.poll_loaded() {
            match loaded {
                Ok(state) if !touched_before_restore.get() => {
                    let selected = state.selected;
                    queue.borrow_mut().clear();
                    queue.borrow_mut().append(state.tracks);
                    if let Some(index) = selected { let _ = queue.borrow_mut().select(index); }
                    refresh_list(&queue, &model);
                    let count = queue.borrow().count();
                    window.set_library_count(format!("{count} tracks").into());
                    window.set_notice(format!("Restored {count} tracks from previous session").into());
                }
                Ok(_) => {} // User action has priority over stale session loading.
                Err(reason) => window.set_notice(format!("Session recovery: {reason}").into()),
            }
        }
        // Local M3U I/O happens on dedicated threads; apply models on UI thread.
        for _ in 0..8 {
            match playlist_rx.try_recv() {
                Ok(PlaylistEvent::Imported(Ok(paths))) => {
                    scanner.cancel();
                    scan_serial.set(0);
                    let size = paths.len();
                    queue.borrow_mut().clear();
                    queue.borrow_mut().append(paths);
                    dirty_for_updates.set(true);
                    refresh_list(&queue, &model);
                    window.set_library_count(format!("{size} tracks").into());
                    window.set_notice(format!("Imported {size} playlist entries").into());
                }
                Ok(PlaylistEvent::Imported(Err(reason))) => {
                    window.set_notice(format!("Playlist import error: {reason}").into());
                }
                Ok(PlaylistEvent::Saved(Ok(count))) => {
                    window.set_notice(format!("Saved {count} M3U8 entries").into());
                }
                Ok(PlaylistEvent::Saved(Err(reason))) => {
                    window.set_notice(format!("Playlist export error: {reason}").into());
                }
                Err(_) => break,
            }
        }
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
                    dirty_for_updates.set(true);
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
            window.set_audio_level(state.audio_level_percent);
            window.set_elapsed_text(format_duration(state.position).into());
            window.set_duration_text(state.duration.map(format_duration).unwrap_or_else(|| "--:--".into()).into());
            window.set_notice(audio::status_text(&state).into());
            if state.transport == Transport::Playing { pending_advance.set(false); }
            if state.transport == Transport::Stopped && state.detail == "Track finished" && !pending_advance.get() {
                pending_advance.set(true);
                if let Some(path) = queue.borrow_mut().next() {
                    dirty_for_updates.set(true);
                    let _ = commands.send(Command::Open(path));
                }
            }
        }
        if dirty_for_updates.replace(false) {
            let snapshot = {
                let q = queue.borrow();
                SavedSession { tracks: q.snapshot(), selected: q.selected_index() }
            };
            session_for_updates.enqueue_save(snapshot);
        }
    });

    ui.run()?;
    scanner_shutdown.shutdown();
    let _ = command_shutdown.send(Command::Shutdown);
    // Flush the latest state and join I/O worker before exiting.
    let final_snapshot = {
        let q = session_shutdown_queue.borrow();
        SavedSession { tracks: q.snapshot(), selected: q.selected_index() }
    };
    session.shutdown(final_snapshot);
    Ok(())
}

fn refresh_list(queue: &Rc<RefCell<PlayQueue>>, rows: &Rc<VecModel<SharedString>>) {
    let preview = queue.borrow().preview(10);
    rows.set_vec(preview.into_iter().map(SharedString::from).collect::<Vec<_>>());
}
