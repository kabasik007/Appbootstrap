//! Local playback worker: all disk/decoder/output-device work runs outside Slint.
//!
//! P1 bootstrap deliberately uses Rodio's CPAL/Symphonia backend for a reliable
//! vertical slice. Custom callback-safe DSP and FFT remain separate P2/P4 tasks.
use crate::dsp::{EqControls, EqSource};
use crate::playback::{
    format_duration, media_title, normalise_volume, seek_position, PlaybackState, Transport,
};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError};
use std::thread;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub enum Command {
    Open(PathBuf),
    TogglePlay,
    Stop,
    SeekPercent(f32),
    SetVolume(f32),
    SetEqBand(usize, f32),
    SetPreamp(f32),
    SetBass(f32),
    SetTreble(f32),
    SetLoudness(bool),
    EnableEq(bool),
    Shutdown,
}

pub struct AudioController {
    pub commands: Sender<Command>,
    pub updates: Receiver<PlaybackState>,
}

pub fn start_audio_worker() -> AudioController {
    let (commands, rx) = mpsc::channel();
    // UI never waits for a status snapshot; if behind, discard snapshots.
    let (updates, tx) = mpsc::sync_channel(16);
    thread::Builder::new()
        .name("zillaplayer-audio-control".to_string())
        .spawn(move || worker_loop(rx, tx))
        .expect("failed to start audio control thread");

    AudioController { commands, updates }
}

fn worker_loop(commands: Receiver<Command>, events: SyncSender<PlaybackState>) {
    let mut stream: Option<OutputStream> = None;
    let mut sink: Option<Sink> = None;
    let mut last_path: Option<PathBuf> = None;
    let mut state = PlaybackState::default();
    let eq = EqControls::new();

    loop {
        match commands.recv_timeout(Duration::from_millis(200)) {
            Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
            Ok(Command::Open(path)) => {
                // Open and decoder validation happen on this worker, NOT on UI.
                state.detail = "Opening local media…".to_string();
                state.transport = Transport::Idle;
                send_snapshot(&events, &state);

                match open_track(&path, &mut stream, &mut sink, state.volume, Arc::clone(&eq)) {
                    Ok(duration) => {
                        state.title = media_title(&path);
                        state.position = Duration::ZERO;
                        state.duration = duration;
                        state.transport = Transport::Playing;
                        state.has_track = true;
                        state.detail = "Playing local media (WASAPI via CPAL)".to_string();
                        last_path = Some(path);
                    }
                    Err(error) => {
                        // If a different track is already playing, a bad new file
                        // must never silence it or pretend that playback stopped.
                        if let Some(previous) = sink.as_ref() {
                            state.transport = if previous.is_paused() {
                                Transport::Paused
                            } else {
                                Transport::Playing
                            };
                            state.detail = format!("Open failed; previous track kept: {error}");
                        } else {
                            state.transport = Transport::Error;
                            state.detail = error;
                            state.position = Duration::ZERO;
                            state.has_track = false;
                            state.duration = None;
                        }
                    }
                }
            }
            Ok(Command::TogglePlay) => {
                if let Some(current) = sink.as_ref() {
                    if current.is_paused() {
                        current.set_volume(0.0);
                        current.play();
                        fade(current, 0.0, state.volume, 60);
                        state.transport = Transport::Playing;
                        state.detail = "Resumed".into();
                    } else {
                        fade(current, current.volume(), 0.0, 55);
                        current.pause();
                        state.transport = Transport::Paused;
                        state.detail = "Paused".into();
                    }
                } else if let Some(path) = last_path.as_ref() {
                    match open_track(path, &mut stream, &mut sink, state.volume, Arc::clone(&eq)) {
                        Ok(duration) => {
                            state.duration = duration;
                            state.position = Duration::ZERO;
                            state.transport = Transport::Playing;
                            state.has_track = true;
                            state.detail = "Playing".into();
                        }
                        Err(error) => {
                            state.transport = Transport::Error;
                            state.detail = error;
                            state.has_track = false;
                        }
                    }
                } else {
                    state.detail = "Choose a local file with Open File first".into();
                }
            }
            Ok(Command::Stop) => {
                if let Some(current) = sink.take() {
                    fade(&current, current.volume(), 0.0, 50);
                    current.stop();
                }
                state.transport = Transport::Stopped;
                state.position = Duration::ZERO;
                state.detail = "Stopped".into();
                state.has_track = last_path.is_some();
            }
            Ok(Command::SeekPercent(percent)) => {
                if let (Some(current), Some(position)) =
                    (sink.as_ref(), seek_position(percent, state.duration))
                {
                    let paused = current.is_paused();
                    if !paused { fade(current, current.volume(), 0.0, 45); }
                    match current.try_seek(position) {
                        Ok(()) => {
                            state.position = position;
                            state.detail = "Seek complete (soft transition)".into();
                        }
                        Err(error) => state.detail = format!("Seek unavailable: {error}"),
                    }
                    if !paused { fade(current, 0.0, state.volume, 65); }
                    else { current.set_volume(state.volume); }
                }
            }
            Ok(Command::SetVolume(percent)) => {
                state.volume = normalise_volume(percent);
                if let Some(current) = sink.as_ref() {
                    if !current.is_paused() {
                        fade(current, current.volume(), state.volume, 20);
                    } else { current.set_volume(state.volume); }
                }
            }
            Ok(Command::SetEqBand(i, db)) => eq.set_band(i, db),
            Ok(Command::SetPreamp(db)) => eq.set_preamp(db),
            Ok(Command::SetBass(db)) => eq.set_bass(db),
            Ok(Command::SetTreble(db)) => eq.set_treble(db),
            Ok(Command::SetLoudness(on)) => eq.loudness.store(on, std::sync::atomic::Ordering::Relaxed),
            Ok(Command::EnableEq(on)) => eq.enabled.store(on, std::sync::atomic::Ordering::Relaxed),
        }

        if let Some(current) = sink.as_ref() {
            if current.empty() {
                // Release the previous sink: TogglePlay can reopen the file.
                sink = None;
                state.transport = Transport::Stopped;
                state.position = state.duration.unwrap_or(Duration::ZERO);
                state.detail = "Track finished".into();
            } else {
                state.position = current.get_pos();
            }
        }
        send_snapshot(&events, &state);
    }
    // Sink stops, then OutputStream ends, when these locals are dropped.
}

fn open_track(
    path: &PathBuf,
    stream: &mut Option<OutputStream>,
    sink: &mut Option<Sink>,
    volume: f32,
    eq: Arc<EqControls>,
) -> Result<Option<Duration>, String> {
    if !path.is_file() {
        return Err(format!("File not found: {}", path.display()));
    }
    let file = File::open(path).map_err(|error| format!("Cannot open file: {error}"))?;
    let source = Decoder::try_from(file)
        .map_err(|error| format!("Unsupported or damaged media: {error}"))?;
    let duration = source.total_duration();

    // Stop previous playback only once the replacement was validated.
    if stream.is_none() {
        *stream = Some(
            OutputStreamBuilder::open_default_stream()
                .map_err(|error| format!("Audio output unavailable: {error}"))?,
        );
    }
    let Some(device) = stream.as_ref() else {
        return Err("Audio output unavailable".into());
    };
    let new_sink = Sink::connect_new(device.mixer());
    new_sink.set_volume(0.0);
    new_sink.append(EqSource::new(source, eq));
    if let Some(previous) = sink.replace(new_sink) {
        fade(&previous, previous.volume(), 0.0, 60);
        previous.stop();
    }
    if let Some(new_sink) = sink.as_ref() {
        fade(new_sink, 0.0, volume, 70);
    }
    Ok(duration)
}

fn send_snapshot(sender: &SyncSender<PlaybackState>, state: &PlaybackState) {
    match sender.try_send(state.clone()) {
        Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
    }
}

/// Format status message for the top-level UI without leaking full local paths.
pub fn status_text(state: &PlaybackState) -> String {
    let position = format_duration(state.position);
    let duration = state.duration.map(format_duration).unwrap_or_else(|| "--:--".into());
    format!("{} | {position}/{duration} | {}", state.label(), state.detail)
}


/// Smooth envelope on the control thread only. Never sleeps on the CPAL callback.
fn fade(sink: &Sink, start: f32, finish: f32, milliseconds: u64) {
    const STEPS: u64 = 10;
    for step in 1..=STEPS {
        let t = step as f32 / STEPS as f32;
        // Raised cosine envelope has zero slope at both ends.
        let weight = (1.0 - (std::f32::consts::PI * t).cos()) * 0.5;
        sink.set_volume(start + (finish - start) * weight);
        thread::sleep(Duration::from_millis(milliseconds / STEPS));
    }
}
