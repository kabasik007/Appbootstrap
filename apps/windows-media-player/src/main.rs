#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod playback;

use audio::{start_audio_worker, AudioController, Command};
use playback::{format_duration, Transport};
use slint::ComponentHandle;
use std::error::Error;
use std::time::Duration;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    let AudioController { commands, updates } = start_audio_worker();

    // Modal native file picker is a user-initiated interaction.
    // All actual file I/O and audio initialization happen in the audio worker.
    let open_commands = commands.clone();
    ui.on_open_file(move || {
        let selected = rfd::FileDialog::new()
            .add_filter("Audio files", &["mp3", "flac", "wav", "ogg", "m4a", "aac"])
            .pick_file();
        if let Some(path) = selected {
            let _ = open_commands.send(Command::Open(path));
        }
    });

    let play_commands = commands.clone();
    ui.on_toggle_play(move || {
        let _ = play_commands.send(Command::TogglePlay);
    });

    let stop_commands = commands.clone();
    ui.on_stop_playback(move || {
        let _ = stop_commands.send(Command::Stop);
    });

    let seek_commands = commands.clone();
    ui.on_seek_to(move |percent| {
        let _ = seek_commands.send(Command::SeekPercent(percent));
    });

    let volume_commands = commands.clone();
    ui.on_set_volume(move |percent| {
        let _ = volume_commands.send(Command::SetVolume(percent));
    });

    // A bounded audio snapshot channel is read only on the Slint event loop.
    // UI does not block on decoder, disk, or audio device operations.
    let weak_ui = ui.as_weak();
    let updates_timer = slint::Timer::default();
    updates_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(200),
        move || {
            let mut latest = None;
            while let Ok(update) = updates.try_recv() {
                latest = Some(update);
            }
            if let (Some(state), Some(ui)) = (latest, weak_ui.upgrade()) {
                ui.set_track_title(state.title.clone().into());
                ui.set_playing(state.transport == Transport::Playing);
                ui.set_has_track(state.has_track);
                ui.set_progress_percent(state.progress_percent());
                ui.set_elapsed_text(format_duration(state.position).into());
                ui.set_duration_text(
                    state.duration
                        .map(format_duration)
                        .unwrap_or_else(|| "--:--".to_owned())
                        .into(),
                );
                ui.set_notice(audio::status_text(&state).into());
            }
        },
    );
    ui.run()?;
    let _ = commands.send(Command::Shutdown);
    Ok(())
}
