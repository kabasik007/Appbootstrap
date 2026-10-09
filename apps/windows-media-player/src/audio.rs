//! Audio control thread. All file opens and format decoding are delegated to
//! decode_worker, which fills a bounded SPSC PCM buffer. The output source
//! reads *only* memory from that buffer, then applies DSP in the mixer path.
use crate::{
    decode_worker,
    dsp::{EqControls, EqSource},
    pcm_ring::PcmRing,
    playback::{
        fade_weight, format_duration, media_title, normalise_volume, seek_position,
        PlaybackState, Transport,
    },
};
use rodio::{OutputStream, OutputStreamBuilder, Sink};
use std::{
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError},
        Arc,
    },
    thread,
    time::Duration,
};

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
    let (commands, incoming) = mpsc::channel();
    let (updates, outgoing) = mpsc::sync_channel(16);
    thread::Builder::new()
        .name("zillaplayer-audio-control".into())
        .spawn(move || worker_loop(incoming, outgoing))
        .expect("failed to spawn audio control thread");
    AudioController { commands, updates }
}

/// The single owner of transport commands, Rodio sink and device stream.
/// No sleep/file open/decoder init here can block the Slint event loop.
fn worker_loop(commands: Receiver<Command>, events: SyncSender<PlaybackState>) {
    let mut output: Option<OutputStream> = None;
    let mut sink: Option<Sink> = None;
    let mut decoder: Option<Arc<PcmRing>> = None;
    let mut file: Option<PathBuf> = None;
    let mut position_offset = Duration::ZERO;
    let mut state = PlaybackState::default();
    let eq = EqControls::new();
    let mut deferred_command: Option<Command> = None;

    loop {
        // Retain FIFO order for non-seek commands while collapsing a burst of
        // adjacent seeks into the most recent target before opening a decoder.
        let incoming = if let Some(saved) = deferred_command.take() {
            Ok(saved)
        } else {
            commands.recv_timeout(Duration::from_millis(150))
        };
        match incoming {
            Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
            Ok(Command::Open(path)) => {
                state.detail = "Opening media on background decoder…".into();
                send_snapshot(&events, &state);
                match install_track(
                    &path, Duration::ZERO, false,
                    &mut output, &mut sink, &mut decoder, state.volume, Arc::clone(&eq),
                ) {
                    Ok(duration) => {
                        file = Some(path.clone());
                        state.title = media_title(&path);
                        state.position = Duration::ZERO;
                        position_offset = Duration::ZERO;
                        state.duration = duration;
                        state.transport = Transport::Playing;
                        state.has_track = true;
                        state.detail = "Playing / buffered decoder → EQ → WASAPI".into();
                    }
                    Err(error) => {
                        if let Some(old) = sink.as_ref() {
                            state.transport = if old.is_paused() {
                                Transport::Paused
                            } else { Transport::Playing };
                            state.detail = format!("Open failed; previous track kept: {error}");
                        } else {
                            state.transport = Transport::Error;
                            state.detail = error;
                            state.has_track = false;
                            state.duration = None;
                            state.position = Duration::ZERO;
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
                } else if let Some(path) = file.as_ref() {
                    match install_track(
                        path, Duration::ZERO, false, &mut output, &mut sink,
                        &mut decoder, state.volume, Arc::clone(&eq),
                    ) {
                        Ok(duration) => {
                            state.duration = duration;
                            state.position = Duration::ZERO;
                            position_offset = Duration::ZERO;
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
                    state.detail = "Open a local audio file to begin".into();
                }
            }
            Ok(Command::Stop) => {
                if let Some(old) = sink.take() {
                    fade(&old, old.volume(), 0.0, 55);
                    old.stop();
                }
                if let Some(ring) = decoder.take() { ring.cancel(); }
                position_offset = Duration::ZERO;
                state.position = Duration::ZERO;
                state.transport = Transport::Stopped;
                state.has_track = file.is_some();
                state.detail = "Stopped".into();
            }
            Ok(Command::SeekPercent(first_percent)) => {
                let percent = collapse_adjacent_seeks(first_percent, &commands, &mut deferred_command);
                if let (Some(path), Some(target)) = (
                    file.as_ref(), seek_position(percent, state.duration),
                ) {
                    let paused = sink.as_ref().is_some_and(Sink::is_paused);
                    // Rather than asking the output callback to seek a decoder,
                    // open a new decoder worker and prebuffer at the target offset.
                    // Failed seek keeps previous playback uninterrupted.
                    match install_track(
                        path, target, paused, &mut output, &mut sink,
                        &mut decoder, state.volume, Arc::clone(&eq),
                    ) {
                        Ok(duration) => {
                            position_offset = target;
                            state.position = target;
                            state.duration = duration;
                            state.transport = if paused { Transport::Paused }
                                else { Transport::Playing };
                            state.detail = "Seek complete (decoder restarted with soft fade)".into();
                        }
                        Err(error) => {
                            state.detail = format!("Seek failed; previous track kept: {error}");
                        }
                    }
                }
            }
            Ok(Command::SetVolume(percent)) => {
                state.volume = normalise_volume(percent);
                if let Some(current) = sink.as_ref() {
                    if current.is_paused() { current.set_volume(state.volume); }
                    else { fade(current, current.volume(), state.volume, 20); }
                }
            }
            Ok(Command::SetEqBand(i, db)) => eq.set_band(i, db),
            Ok(Command::SetPreamp(db)) => eq.set_preamp(db),
            Ok(Command::SetBass(db)) => eq.set_bass(db),
            Ok(Command::SetTreble(db)) => eq.set_treble(db),
            Ok(Command::SetLoudness(on)) => eq.set_loudness(on),
            Ok(Command::EnableEq(on)) => eq.set_enabled(on),
        }

        if let Some(current) = sink.as_ref() {
            if current.empty() {
                sink = None;
                if let Some(ring) = decoder.take() { ring.cancel(); }
                state.position = state.duration.unwrap_or(position_offset);
                state.transport = Transport::Stopped;
                state.detail = "Track finished".into();
            } else {
                // Sink position measures playback since the last seek/open.
                state.position = position_offset.saturating_add(current.get_pos());
                if let Some(duration) = state.duration {
                    state.position = state.position.min(duration);
                }
            }
        }
        state.buffer_starvations = decoder.as_ref().map_or(0, |buffer| buffer.underruns());
        state.audio_level_percent = if state.transport == Transport::Playing {
            decoder.as_ref().map_or(0.0, |buffer| buffer.peak_percent())
        } else { 0.0 };
        send_snapshot(&events, &state);
    }

    if let Some(active) = decoder.take() { active.cancel(); }
    if let Some(active) = sink.take() { active.stop(); }
}

#[allow(clippy::too_many_arguments)]
fn install_track(
    path: &PathBuf,
    offset: Duration,
    pause_new: bool,
    stream: &mut Option<OutputStream>,
    sink: &mut Option<Sink>,
    active_decoder: &mut Option<Arc<PcmRing>>,
    volume: f32,
    eq: Arc<EqControls>,
) -> Result<Option<Duration>, String> {
    // Disk I/O and all decoder work take place on the decode worker.
    // This call only waits for metadata and a bounded initial prebuffer.
    let prepared = decode_worker::prepare(path.clone(), offset)?;
    let duration = prepared.duration;
    if stream.is_none() {
        *stream = Some(OutputStreamBuilder::open_default_stream()
            .map_err(|err| format!("Audio output unavailable: {err}"))?);
    }
    let device = stream.as_ref().ok_or("Audio output unavailable")?;

    // Stop the previous sample-rate's DSP before designing next coefficients.
    if let Some(previous) = sink.as_ref() {
        if !previous.is_paused() {
            fade(previous, previous.volume(), 0.0, 60);
            previous.pause();
        }
    }

    let new_sink = Sink::connect_new(device.mixer());
    new_sink.set_volume(0.0);
    new_sink.append(EqSource::new(prepared.source, eq));
    if pause_new { new_sink.pause(); }
    if let Some(ring) = active_decoder.replace(prepared.cancel) {
        ring.cancel();
    }
    if let Some(old) = sink.replace(new_sink) {
        old.stop();
    }
    if let Some(current) = sink.as_ref() {
        if pause_new { current.set_volume(volume); }
        else { fade(current, 0.0, volume, 70); }
    }
    Ok(duration)
}

/// Coalesce only consecutive seek commands; preserve FIFO for all other input.
fn collapse_adjacent_seeks(
    first: f32,
    commands: &Receiver<Command>,
    deferred: &mut Option<Command>,
) -> f32 {
    let mut target = first;
    while let Ok(next) = commands.try_recv() {
        match next {
            Command::SeekPercent(percent) => target = percent,
            other => { *deferred = Some(other); break; }
        }
    }
    target
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rapid_seeks_collapse_without_losing_stop() {
        let (tx, rx) = mpsc::channel();
        tx.send(Command::SeekPercent(10.)).unwrap();
        tx.send(Command::SeekPercent(42.)).unwrap();
        tx.send(Command::SeekPercent(88.)).unwrap();
        tx.send(Command::Stop).unwrap();
        let Command::SeekPercent(first) = rx.recv().unwrap() else { panic!("first event") };
        let mut deferred = None;
        let pct = collapse_adjacent_seeks(first, &rx, &mut deferred);
        assert_eq!(pct, 88.);
        assert!(matches!(deferred, Some(Command::Stop)));
    }
    #[test]
    fn eq_update_is_not_discarded_by_seek_collapse() {
        let (tx, rx) = mpsc::channel();
        tx.send(Command::SetEqBand(16, 3.)).unwrap();
        let mut pending = None;
        assert_eq!(collapse_adjacent_seeks(35., &rx, &mut pending), 35.);
        assert!(matches!(pending, Some(Command::SetEqBand(16, gain)) if gain == 3.));
    }
}

fn send_snapshot(sender: &SyncSender<PlaybackState>, state: &PlaybackState) {
    match sender.try_send(state.clone()) {
        Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
    }
}

pub fn status_text(state: &PlaybackState) -> String {
    let position = format_duration(state.position);
    let duration = state.duration.map(format_duration).unwrap_or_else(|| "--:--".into());
    if state.buffer_starvations == 0 {
        format!("{} | {position}/{duration} | {}", state.label(), state.detail)
    } else {
        format!(
            "{} | {position}/{duration} | {} | PCM starvation blocks: {}",
            state.label(), state.detail, state.buffer_starvations
        )
    }
}

/// Raised-cosine fades run only on the transport control worker.
fn fade(sink: &Sink, start: f32, finish: f32, milliseconds: u64) {
    const STEPS: u64 = 10;
    for step in 1..=STEPS {
        let t = step as f32 / STEPS as f32;
        let weight = fade_weight(t);
        sink.set_volume(start + (finish - start) * weight);
        thread::sleep(Duration::from_millis(milliseconds / STEPS));
    }
}
