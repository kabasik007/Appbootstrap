// Alpha Windows test build uses console subsystem so --scan-worker can reliably
// inherit JSONL stdout pipes. Split GUI/scanner binaries before public release.

mod audio;
mod decode_worker;
mod dsp;
mod downloader;
mod download_analysis;
mod library;
mod playback;
mod pcm_ring;
mod playlist;
mod presets;
mod queue;
mod roadmap;
mod session;
mod visualizer;

use audio::{start_audio_worker, AudioController, Command};
use library::{LibraryScanner, ScanEvent};
use downloader::{Downloader, Format};
use download_analysis::{Preview, spawn_preview, row_title};
use playback::{format_duration, Transport};
use queue::PlayQueue;
use presets::EqPreset;
use session::{NamedPlaylist, SavedSession, SessionStore};
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
const LIBRARY_PAGE_SIZE: usize = 12;

#[derive(Default)]
struct LibraryView {
    query: String,
    page: usize,
    visible: Vec<usize>,
}

struct LibraryPresentation {
    queue: Rc<RefCell<PlayQueue>>,
    rows: Rc<VecModel<SharedString>>,
    view: RefCell<LibraryView>,
    window: slint::Weak<AppWindow>,
}

fn refresh_library(library: &LibraryPresentation) {
    let entries = library.queue.borrow().snapshot();
    let mut view = library.view.borrow_mut();
    let query = view.query.trim().to_lowercase();
    let matches: Vec<usize> = entries.iter().enumerate()
        .filter_map(|(i, path)| {
            if query.is_empty() || path.file_name().unwrap_or_default()
                .to_string_lossy().to_lowercase().contains(&query) { Some(i) } else { None }
        }).collect();
    let pages = matches.len().saturating_sub(1) / LIBRARY_PAGE_SIZE + 1;
    view.page = view.page.min(pages - 1);
    let start = view.page * LIBRARY_PAGE_SIZE;
    view.visible = matches.iter().skip(start).take(LIBRARY_PAGE_SIZE).copied().collect();
    let filenames: Vec<SharedString> = view.visible.iter()
        .map(|i| SharedString::from(entries[*i].file_name().unwrap_or_default()
            .to_string_lossy().as_ref())).collect();
    library.rows.set_vec(filenames);
    if let Some(window) = library.window.upgrade() {
        window.set_library_page_info(format!("{}/{} · {}",
            view.page+1, pages, matches.len()).into());
        window.set_library_count(format!("{} {}",
            entries.len(), if window.get_uk() { "треків" } else { "tracks" }).into());
    }
}

fn update_playlist_names(names: &[NamedPlaylist], model: &VecModel<SharedString>) {
    model.set_vec(names.iter().map(|p| SharedString::from(
        format!("♫ {} ({})", p.name, p.tracks.len()))).collect::<Vec<_>>());
}

fn session_snapshot(
    queue: &PlayQueue, playlists: &[NamedPlaylist],
    all_presets: &[EqPreset], current_eq: &EqPreset,
) -> SavedSession {
    SavedSession {
        tracks: queue.snapshot(), selected: queue.selected_index(),
        playlists: playlists.to_vec(),
        presets: all_presets.iter().skip(presets::factory().len()).cloned().collect(),
        last_eq: current_eq.clone(),
    }
}


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
    let library = Rc::new(LibraryPresentation {
        queue: Rc::clone(&queue), rows: Rc::clone(&model),
        view: RefCell::new(LibraryView::default()), window: ui.as_weak(),
    });
    refresh_library(&library);
    let named_playlists: Rc<RefCell<Vec<NamedPlaylist>>> = Rc::new(RefCell::new(Vec::new()));
    let selected_playlist = Rc::new(Cell::new(None::<usize>));
    let playlist_names: Rc<VecModel<SharedString>> = Rc::new(VecModel::default());
    ui.set_playlist_names(playlist_names.clone().into());
    let all_presets: Rc<RefCell<Vec<EqPreset>>> = Rc::new(RefCell::new(presets::factory()));
    let preset_names: Rc<VecModel<SharedString>> = Rc::new(VecModel::from(
        all_presets.borrow().iter().map(|p| SharedString::from(p.name.as_str()))
            .collect::<Vec<_>>()
    ));
    ui.set_preset_names(preset_names.clone().into());
    let current_eq = Rc::new(RefCell::new(EqPreset::flat()));
    let eq_values: Rc<VecModel<f32>> = Rc::new(VecModel::from(vec![0.0; 31]));
    ui.set_eq_bands(eq_values.clone().into());
    let eq_programmatic = Rc::new(Cell::new(false));
    let touched_eq = Rc::new(Cell::new(false));
    // Single background session I/O worker. All disk reads/writes stay off UI.
    let session = Rc::new(SessionStore::start()?);
    let session_dirty = Rc::new(Cell::new(false));
    let user_touched_queue = Rc::new(Cell::new(false));
    // The download actor persists its own queue and owns all yt-dlp/FFmpeg
    // children independently of Slint and the audio engine.
    let downloader = Rc::new(Downloader::start());
    ui.set_download_folder(downloader::suggested_folder().to_string_lossy().to_string().into());
    let download_rows: Rc<VecModel<SharedString>> = Rc::new(VecModel::default());
    ui.set_download_job_rows(download_rows.clone().into());
    let preview_rows: Rc<VecModel<SharedString>> = Rc::new(VecModel::default());
    ui.set_download_preview_rows(preview_rows.clone().into());
    let preview_state: Rc<RefCell<Option<Preview>>> = Rc::new(RefCell::new(None));
    let pending_preview = Rc::new(RefCell::new(None::<mpsc::Receiver<Result<Preview,String>>>));
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
    let lib = library.clone();
    ui.on_open_file(move || {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Audio", &["mp3","flac","wav","ogg","m4a","aac","opus"]).pick_file()
        {
            touched.set(true);
            dirty.set(true);
            q.borrow_mut().append_selected(path.clone());
            refresh_library(&lib);
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
    let lib = library.clone();
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
    let lib = library.clone();
    ui.on_play_library_track(move |index| {
        if index >= 0 {
            let actual = lib.view.borrow().visible.get(index as usize).copied();
            if let Some(path) = actual.and_then(|i| q.borrow_mut().select(i)) {
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
    let eq_ref = current_eq.clone();
    let suppress = eq_programmatic.clone();
    let values = eq_values.clone();
    let dirty = session_dirty.clone();
    let touched = touched_eq.clone();
    ui.on_set_eq_band(move |index, db| {
        if !suppress.get() && index >= 0 && (index as usize) < EQ_BAND_COUNT && db.is_finite() {
            let level = db.clamp(-12.0, 12.0);
            eq_ref.borrow_mut().bands[index as usize] = level;
            values.set_row_data(index as usize, level);
            touched.set(true);
            dirty.set(true);
            let _ = cmd.send(Command::SetEqBand(index as usize, db));
        }
    });
    let cmd = commands.clone();
    let eq_ref = current_eq.clone();
    let suppress = eq_programmatic.clone();
    let weak = ui.as_weak();
    let dirty = session_dirty.clone();
    let touched = touched_eq.clone();
    ui.on_set_preamp(move |db| {
        if !suppress.get() && db.is_finite() {
            eq_ref.borrow_mut().preamp = db.clamp(-18.0, 6.0);
            if let Some(window) = weak.upgrade() { window.set_eq_preamp(db.clamp(-18.0, 6.0)); }
            touched.set(true);
            dirty.set(true);
            let _ = cmd.send(Command::SetPreamp(db));
        }
    });
    let cmd = commands.clone();
    ui.on_set_bass(move |db| { let _ = cmd.send(Command::SetBass(db)); });
    let cmd = commands.clone();
    ui.on_set_treble(move |db| { let _ = cmd.send(Command::SetTreble(db)); });
    let cmd = commands.clone();
    ui.on_set_loudness(move |on| { let _ = cmd.send(Command::SetLoudness(on)); });
    let cmd = commands.clone();
    ui.on_enable_eq(move |on| { let _ = cmd.send(Command::EnableEq(on)); });

    // Factory presets + saved user presets share one compact UI model.
    let cmd = commands.clone();
    let values = eq_values.clone();
    let eq_ref = current_eq.clone();
    let all = all_presets.clone();
    let suppress = eq_programmatic.clone();
    let touched = touched_eq.clone();
    let dirty = session_dirty.clone();
    let weak = ui.as_weak();
    ui.on_apply_eq_preset(move |index| {
        if index < 0 { return; }
        let Some(preset) = all.borrow().get(index as usize).cloned() else { return; };
        eq_ref.replace(preset.clone());
        suppress.set(true);
        values.set_vec(preset.bands.to_vec());
        if let Some(window) = weak.upgrade() {
            window.set_eq_preamp(preset.preamp);
            window.set_notice(format!("EQ: {}", preset.name).into());
        }
        suppress.set(false);
        touched.set(true);
        dirty.set(true);
        let _ = cmd.send(Command::SetEqCurve(preset.bands, preset.preamp));
    });

    let presets_all = all_presets.clone();
    let names = preset_names.clone();
    let eq_ref = current_eq.clone();
    let dirty = session_dirty.clone();
    let touched = touched_eq.clone();
    let weak = ui.as_weak();
    ui.on_save_eq_preset(move |name| {
        let label = name.trim();
        let Some(window) = weak.upgrade() else { return; };
        if label.is_empty() {
            window.set_notice("Введіть назву пресету".into());
            return;
        }
        let mut current = eq_ref.borrow().clone();
        current.name = label.into();
        current = current.sanitize();
        let mut all = presets_all.borrow_mut();
        let builtins = presets::factory().len();
        if let Some(pos) = all.iter().enumerate().skip(builtins)
            .position(|(_, p)| p.name == current.name) {
            all[builtins + pos] = current.clone();
        } else if all.len() < builtins + 24 {
            all.push(current.clone());
        } else {
            window.set_notice("Ліміт 24 користувацьких пресетів".into());
            return;
        }
        names.set_vec(all.iter().map(|p| SharedString::from(p.name.as_str())).collect::<Vec<_>>());
        touched.set(true);
        dirty.set(true);
        window.set_custom_preset_name("".into());
        window.set_notice(format!("Пресет збережено: {}", current.name).into());
    });

    let lib = library.clone();
    ui.on_search_library(move |term| {
        let mut view = lib.view.borrow_mut();
        view.query = term.to_string();
        view.page = 0;
        drop(view);
        refresh_library(&lib);
    });
    let lib = library.clone();
    ui.on_library_page(move |direction| {
        {
            let mut view = lib.view.borrow_mut();
            if direction < 0 { view.page = view.page.saturating_sub(1); }
            else { view.page = view.page.saturating_add(1); }
        }
        refresh_library(&lib);
    });

    let playlists = named_playlists.clone();
    let selected = selected_playlist.clone();
    let names = playlist_names.clone();
    let q = queue.clone();
    let dirty = session_dirty.clone();
    let touched = user_touched_queue.clone();
    let weak = ui.as_weak();
    ui.on_create_playlist(move |name| {
        let Some(window) = weak.upgrade() else { return; };
        let title = name.trim().chars().take(64).collect::<String>();
        if title.is_empty() { window.set_notice("Введіть назву плейлиста".into()); return; }
        let mut all = playlists.borrow_mut();
        if all.iter().any(|p| p.name.eq_ignore_ascii_case(&title)) {
            window.set_notice("Плейлист із такою назвою вже існує".into()); return;
        }
        if all.len() >= 64 { window.set_notice("Максимум 64 плейлисти".into()); return; }
        all.push(NamedPlaylist { name: title.clone(), tracks: q.borrow().snapshot() });
        selected.set(Some(all.len() - 1));
        update_playlist_names(&all, &names);
        window.set_active_playlist_name(title.clone().into());
        window.set_new_playlist_name("".into());
        window.set_notice(format!("Створено плейлист: {title}").into());
        dirty.set(true);
        touched.set(true);
    });

    let lib = library.clone();
    let q = queue.clone();
    let lists = named_playlists.clone();
    let selected = selected_playlist.clone();
    let touched = user_touched_queue.clone();
    let dirty = session_dirty.clone();
    let weak = ui.as_weak();
    ui.on_load_playlist(move |index| {
        if index < 0 { return; }
        let Some(item) = lists.borrow().get(index as usize).cloned() else { return; };
        selected.set(Some(index as usize));
        q.borrow_mut().clear();
        q.borrow_mut().append(item.tracks);
        if let Some(window) = weak.upgrade() {
            window.set_active_playlist_name(item.name.clone().into());
            window.set_notice(format!("Відкрито плейлист: {}", item.name).into());
        }
        touched.set(true);
        dirty.set(true);
        lib.view.borrow_mut().page = 0;
        refresh_library(&lib);
    });

    let q = queue.clone();
    let lists = named_playlists.clone();
    let selected = selected_playlist.clone();
    let names = playlist_names.clone();
    let dirty = session_dirty.clone();
    let weak = ui.as_weak();
    ui.on_save_active_playlist(move || {
        let Some(index) = selected.get() else { return; };
        let mut all = lists.borrow_mut();
        if let Some(item) = all.get_mut(index) {
            item.tracks = q.borrow().snapshot();
            update_playlist_names(&all, &names);
            dirty.set(true);
            if let Some(window) = weak.upgrade() {
                window.set_notice("Плейлист оновлено".into());
            }
        }
    });

    let lists = named_playlists.clone();
    let selected = selected_playlist.clone();
    let names = playlist_names.clone();
    let dirty = session_dirty.clone();
    let weak = ui.as_weak();
    ui.on_delete_active_playlist(move || {
        let Some(index) = selected.get() else { return; };
        let mut all = lists.borrow_mut();
        if index >= all.len() { return; }
        all.remove(index);
        selected.set(None);
        update_playlist_names(&all, &names);
        dirty.set(true);
        if let Some(window) = weak.upgrade() {
            window.set_active_playlist_name("".into());
            window.set_notice("Плейлист видалено, поточну чергу збережено".into());
        }
    });

    // URL edits or mode changes must not leave a misleading old preview.
    let state = preview_state.clone();
    let pending = pending_preview.clone();
    let rows = preview_rows.clone();
    let weak = ui.as_weak();
    ui.on_invalidate_download_preview(move || {
        state.borrow_mut().take();
        pending.borrow_mut().take();
        rows.set_vec(Vec::new());
        if let Some(window) = weak.upgrade() {
            window.set_download_preview_status("".into());
        }
    });

    // Analysis is metadata-only, async and independent of audio playback.
    let weak = ui.as_weak();
    let pending = pending_preview.clone();
    let state = preview_state.clone();
    let model = preview_rows.clone();
    ui.on_analyze_download(move || {
        let Some(window) = weak.upgrade() else { return; };
        let url = window.get_download_url().to_string();
        let playlist = window.get_download_playlist();
        state.borrow_mut().take();
        model.set_vec(Vec::new());
        window.set_download_preview_status("Аналіз метаданих у фоновому процесі...".into());
        window.set_download_status("Аналіз... Це може зайняти до 35 секунд.".into());
        pending.replace(Some(spawn_preview(url, playlist)));
    });

    let weak = ui.as_weak();
    let state = preview_state.clone();
    let model = preview_rows.clone();
    ui.on_toggle_download_preview(move |index| {
        if index < 0 { return; }
        let mut active = state.borrow_mut();
        let Some(preview) = active.as_mut() else { return; };
        if let Some(item) = preview.items.get_mut(index as usize) {
            item.selected = !item.selected;
            model.set_row_data(index as usize, row_title(item).into());
            if let Some(window) = weak.upgrade() {
                let checked = preview.items.iter().filter(|x|x.selected).count();
                window.set_download_preview_status(format!(
                    "{} · обрано {} з {}", preview.title, checked, preview.items.len()
                ).into());
            }
        }
    });

    let weak = ui.as_weak();
    let state = preview_state.clone();
    let rows = preview_rows.clone();
    ui.on_select_download_preview(move |select_all| {
        let mut preview = state.borrow_mut();
        let Some(list) = preview.as_mut() else { return; };
        for item in list.items.iter_mut() { item.selected = select_all; }
        rows.set_vec(list.items.iter().map(|i| SharedString::from(row_title(i)))
            .collect::<Vec<_>>());
        if let Some(window) = weak.upgrade() {
            window.set_download_preview_status(format!(
                "{} · обрано {} із {}", list.title,
                if select_all { list.items.len() } else { 0 }, list.items.len()
            ).into());
        }
    });

    ui.on_open_download_help(move || {
        // Open only our own bundled text document; never execute user-supplied
        // download URLs or launch a shell.
        if let Ok(binary) = std::env::current_exe() {
            if let Some(folder) = binary.parent() {
                let document = folder.join("README_DOWNLOADS_UA.txt");
                if document.is_file() {
                    #[cfg(windows)]
                    { let _ = std::process::Command::new("notepad.exe").arg(document).spawn(); }
                }
            }
        }
    });

    let dl = downloader.clone();
    let preview = preview_state.clone();
    let weak = ui.as_weak();
    ui.on_enqueue_download(move || {
        let Some(window) = weak.upgrade() else { return; };
        let url = window.get_download_url().to_string();
        let folder = PathBuf::from(window.get_download_folder().as_str());
        let profile = Format::by_index(window.get_download_profile());
        let playlist = window.get_download_playlist();
        let selected_items = {
            let state = preview.borrow();
            match state.as_ref().filter(|p| p.source_url == url && p.is_playlist == playlist) {
                Some(p) if playlist => {
                    let chosen = p.items.iter().filter(|x|x.selected)
                        .map(|x|x.position).collect::<Vec<_>>();
                    if chosen.is_empty() {
                        window.set_download_status("Спочатку оберіть хоча б один трек".into());
                        return;
                    }
                    chosen
                }
                _ => Vec::new(),
            }
        };
        dl.queue(url, profile, playlist, selected_items, folder);
        window.set_download_status("Додано запит. Перевірка URL та інструментів у фоновому потоці...".into());
    });
    let dl = downloader.clone();
    ui.on_check_download_tools(move || { dl.probe(); });
    let dl = downloader.clone();
    ui.on_clear_download_history(move || { dl.clear_finished(); });
    let dl = downloader.clone();
    ui.on_cancel_download(move |index| {
        if index >= 0 { dl.cancel(index as usize); }
    });
    let dl = downloader.clone();
    ui.on_retry_download(move |index| {
        if index >= 0 { dl.retry(index as usize); }
    });
    let weak = ui.as_weak();
    ui.on_choose_download_folder(move || {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            if let Some(window) = weak.upgrade() {
                window.set_download_folder(path.to_string_lossy().to_string().into());
            }
        }
    });

    // UI only: consume ready messages without blocking. No scanning or decoding.
    let scanner_shutdown = scanner.clone();
    let command_shutdown = commands.clone();
    let session_for_updates = Rc::clone(&session);
    let dirty_for_updates = Rc::clone(&session_dirty);
    let touched_before_restore = Rc::clone(&user_touched_queue);
    let session_shutdown_queue = Rc::clone(&queue);
    let persist_playlists = named_playlists.clone();
    let persist_presets = all_presets.clone();
    let persist_eq = current_eq.clone();
    let restore_eq_touched = touched_eq.clone();
    let restore_eq_bands = eq_values.clone();
    let restore_suppress = eq_programmatic.clone();
    let restore_presets_model = preset_names.clone();
    let restore_playlist_model = playlist_names.clone();
    let restore_selected = selected_playlist.clone();
    let restore_commands = commands.clone();
    let library_for_timer = library.clone();
    let dl_for_timer = downloader.clone();
    let dl_rows = download_rows.clone();
    let pending_analysis = pending_preview.clone();
    let analysis_state = preview_state.clone();
    let analysis_rows = preview_rows.clone();
    let weak = ui.as_weak();
    let timer = slint::Timer::default();
    let pending_advance = Rc::new(Cell::new(false));
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(200), move || {
        let Some(window) = weak.upgrade() else { return };
        // Poll analyzer without waiting. The result is owned by Slint UI only;
        // download jobs receive immutable selected playlist indices.
        let finished = pending_analysis.borrow().as_ref().and_then(|rx| {
            match rx.try_recv() {
                Ok(item) => Some(item),
                Err(mpsc::TryRecvError::Disconnected) =>
                    Some(Err("Потік аналізу завершився несподівано".into())),
                Err(mpsc::TryRecvError::Empty) => None,
            }
        });
        if let Some(result) = finished {
            pending_analysis.borrow_mut().take();
            match result {
                Ok(preview) => {
                    let total = preview.items.len();
                    window.set_download_preview_status(format!(
                        "{} · обрано {}/{}", preview.title, total, total
                    ).into());
                    analysis_rows.set_vec(preview.items.iter()
                        .map(|item| SharedString::from(row_title(item))).collect::<Vec<_>>());
                    analysis_state.replace(Some(preview));
                    window.set_download_status(format!("Знайдено позицій: {total}").into());
                }
                Err(err) => {
                    window.set_download_preview_status(format!("Помилка аналізу: {err}").into());
                    window.set_download_status(format!("Помилка аналізу: {err}").into());
                    analysis_rows.set_vec(Vec::new());
                }
            }
        }
        // The supervisor runs independently; reading this snapshot never
        // invokes yt-dlp/FFmpeg or blocks the sound output.
        if window.get_active_page().as_str() == "Downloads" {
            let snapshot = dl_for_timer.snapshot();
            window.set_download_tool_status(snapshot.tools.into());
            window.set_download_status(snapshot.message.into());
            let rows: Vec<SharedString> = snapshot.jobs.iter().map(|job| {
                SharedString::from(format!("#{}  {}  {}{}  {:.1}%  |  {}",
                    job.id, job.format.label(),
                    if job.playlist { "[Плейлист] " } else { "" },
                    job.phase.as_str(), job.progress, job.detail))
            }).collect();
            if rows.len() != dl_rows.row_count() ||
               rows.iter().enumerate().any(|(i, row)| dl_rows.row_data(i).as_ref() != Some(row)) {
                dl_rows.set_vec(rows);
            }
        }
        // Completed authorized local audio files can join the local queue.
        let finished_files = dl_for_timer.take_imported();
        if !finished_files.is_empty() {
            let mut q = queue.borrow_mut();
            let mut already: std::collections::HashSet<PathBuf> =
                q.snapshot().into_iter().collect();
            let mut new_files = Vec::new();
            for file in finished_files {
                if already.insert(file.clone()) { new_files.push(file); }
            }
            let count = new_files.len();
            q.append(new_files);
            drop(q);
            if count > 0 {
                dirty_for_updates.set(true);
                refresh_library(&library_for_timer);
                window.set_notice(format!("Додано аудіофайлів до бібліотеки: {count}").into());
            }
        }
        // Restore only if user has not already started a scan or changed the queue.
        // Loading happens in the persistence worker; nothing reads disk here.
        if let Some(loaded) = session_for_updates.poll_loaded() {
            match loaded {
                Ok(state) => {
                    persist_playlists.replace(state.playlists);
                    update_playlist_names(&persist_playlists.borrow(), &restore_playlist_model);
                    restore_selected.set(None);
                    let mut all = presets::factory();
                    all.extend(state.presets);
                    persist_presets.replace(all);
                    restore_presets_model.set_vec(persist_presets.borrow().iter()
                        .map(|p| SharedString::from(p.name.as_str())).collect::<Vec<_>>());
                    if !restore_eq_touched.get() {
                        let eq = state.last_eq;
                        persist_eq.replace(eq.clone());
                        restore_suppress.set(true);
                        restore_eq_bands.set_vec(eq.bands.to_vec());
                        window.set_eq_preamp(eq.preamp);
                        restore_suppress.set(false);
                        let _ = restore_commands.send(Command::SetEqCurve(eq.bands, eq.preamp));
                    }
                    if !touched_before_restore.get() {
                        let selected = state.selected;
                        queue.borrow_mut().clear();
                        queue.borrow_mut().append(state.tracks);
                        if let Some(index) = selected { let _ = queue.borrow_mut().select(index); }
                        refresh_library(&library_for_timer);
                        window.set_notice("Попередню сесію відновлено".into());
                    }
                }
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
                    refresh_library(&library_for_timer);
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
                    refresh_library(&library_for_timer);
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
            let snapshot = session_snapshot(
                &queue.borrow(), &persist_playlists.borrow(),
                &persist_presets.borrow(), &persist_eq.borrow()
            );
            session_for_updates.enqueue_save(snapshot);
        }
    });

    ui.run()?;
    downloader.shutdown();
    scanner_shutdown.shutdown();
    let _ = command_shutdown.send(Command::Shutdown);
    // Flush the latest state and join I/O worker before exiting.
    let final_snapshot = session_snapshot(
        &session_shutdown_queue.borrow(), &named_playlists.borrow(),
        &all_presets.borrow(), &current_eq.borrow()
    );
    session.shutdown(final_snapshot);
    Ok(())
}

