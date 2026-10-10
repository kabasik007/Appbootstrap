//! Zilla Studio: isolated practice synthesizer and 4-channel live mixer.
//! Notes are procedural PCM sources sent to a *separate* Rodio output stream.
//! No recording, DAW claims, plug-ins, filesystem I/O or UI work on the output callback.
use rodio::{ChannelCount, OutputStreamBuilder, SampleRate, Source};
use std::{
    f32::consts::TAU,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::{self, RecvTimeoutError, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub const CHANNEL_NAMES: [&str; 4] = ["Piano", "Bass", "Lead", "Pad"];
const CHANNELS: usize = 4;
const DEFAULT_RATE: u32 = 48_000;

#[derive(Clone, Copy, Debug)]
enum StudioCommand {
    Note { channel: usize, pitch: u8, velocity: f32 },
    Metronome(bool),
    Bpm(u16),
    Shutdown,
}

struct MixChannel {
    gain: AtomicU32,
    pan: AtomicU32,
    muted: AtomicBool,
}
impl MixChannel {
    fn new() -> Self {
        Self {
            gain: AtomicU32::new(0.65f32.to_bits()),
            pan: AtomicU32::new(0f32.to_bits()),
            muted: AtomicBool::new(false),
        }
    }
    fn gain(&self) -> f32 {
        if self.muted.load(Ordering::Relaxed) { 0.0 }
        else { f32::from_bits(self.gain.load(Ordering::Relaxed)) }
    }
    fn pan(&self) -> f32 {
        f32::from_bits(self.pan.load(Ordering::Relaxed))
    }
}
#[derive(Clone)]
struct MixSettings {
    channels: [Arc<MixChannel>; CHANNELS],
    master: Arc<AtomicU32>,
}
impl MixSettings {
    fn new() -> Self {
        Self {
            channels: std::array::from_fn(|_| Arc::new(MixChannel::new())),
            master: Arc::new(AtomicU32::new(0.65f32.to_bits())),
        }
    }
}

/// Independent studio worker. Loading a 50k-track library, FFmpeg downloads or
/// track seeks cannot block the synth command channel.
pub struct Studio {
    tx: Sender<StudioCommand>,
    settings: MixSettings,
    thread: Mutex<Option<JoinHandle<()>>>,
}
impl Studio {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        let settings = MixSettings::new();
        let worker_settings = settings.clone();
        let worker = thread::Builder::new().name("zilla-studio-synth".into())
            .spawn(move || {
                let mut stream = None;
                let mut bpm = 100u16;
                let mut metronome = false;
                let mut next_tick = Instant::now();
                loop {
                    match rx.recv_timeout(Duration::from_millis(18)) {
                        Ok(StudioCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                        Ok(StudioCommand::Note { channel, pitch, velocity }) => {
                            if channel >= CHANNELS || !(21..=108).contains(&pitch) { continue; }
                            if stream.is_none() {
                                stream = OutputStreamBuilder::open_default_stream().ok();
                            }
                            if let Some(device) = stream.as_ref() {
                                let rate = device.config().sample_rate();
                                device.mixer().add(NoteSource::new(
                                    rate, channel, pitch, velocity, worker_settings.clone(),
                                ));
                            }
                        }
                        Ok(StudioCommand::Bpm(value)) => {
                            bpm = value.clamp(40, 240);
                            next_tick = Instant::now();
                        }
                        Ok(StudioCommand::Metronome(enabled)) => {
                            metronome = enabled;
                            next_tick = Instant::now();
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                    if metronome && Instant::now() >= next_tick {
                        if stream.is_none() {
                            stream = OutputStreamBuilder::open_default_stream().ok();
                        }
                        if let Some(device) = stream.as_ref() {
                            device.mixer().add(NoteSource::click(device.config().sample_rate()));
                        }
                        next_tick = Instant::now() + Duration::from_secs_f64(60. / bpm as f64);
                    }
                }
            }).ok();
        Self { tx, settings, thread: Mutex::new(worker) }
    }
    pub fn note(&self, channel: usize, pitch: u8, velocity: f32) {
        if channel < CHANNELS && (21..=108).contains(&pitch) && velocity.is_finite() {
            let _ = self.tx.send(StudioCommand::Note { channel, pitch, velocity: velocity.clamp(0., 1.) });
        }
    }
    pub fn gain(&self, channel: usize, value: f32) {
        if let Some(mix) = self.settings.channels.get(channel) {
            if value.is_finite() {
                mix.gain.store(value.clamp(0., 1.).to_bits(), Ordering::Relaxed);
            }
        }
    }
    pub fn pan(&self, channel: usize, value: f32) {
        if let Some(mix) = self.settings.channels.get(channel) {
            if value.is_finite() {
                mix.pan.store(value.clamp(-1., 1.).to_bits(), Ordering::Relaxed);
            }
        }
    }
    pub fn mute(&self, channel: usize, value: bool) {
        if let Some(mix) = self.settings.channels.get(channel) {
            mix.muted.store(value, Ordering::Relaxed);
        }
    }
    pub fn master(&self, value: f32) {
        if value.is_finite() {
            self.settings.master.store(value.clamp(0.,1.).to_bits(), Ordering::Relaxed);
        }
    }
    pub fn metronome(&self, enabled: bool) { let _ = self.tx.send(StudioCommand::Metronome(enabled)); }
    pub fn bpm(&self, value: u16) { let _ = self.tx.send(StudioCommand::Bpm(value)); }
    pub fn shutdown(&self) {
        let _ = self.tx.send(StudioCommand::Shutdown);
        if let Some(worker) = self.thread.lock().unwrap_or_else(|e|e.into_inner()).take() {
            let _ = worker.join();
        }
    }
}

pub fn pitch_label(pitch: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
    format!("{}{}", NAMES[pitch as usize % 12], i16::from(pitch)/12-1)
}

pub struct Practice {
    steps: [u8; 8],
    position: usize,
    hits: u32,
    attempts: u32,
}
impl Practice {
    pub fn new() -> Self {
        Self { steps: [60,62,64,65,67,69,71,72], position: 0, hits: 0, attempts: 0 }
    }
    pub fn target(&self) -> u8 { self.steps[self.position] }
    pub fn score(&self) -> (u32,u32) { (self.hits,self.attempts) }
    pub fn press(&mut self, pitch: u8) -> bool {
        self.attempts = self.attempts.saturating_add(1);
        if pitch != self.target() { return false; }
        self.hits = self.hits.saturating_add(1);
        self.position = (self.position + 1) % self.steps.len();
        true
    }
    pub fn reset(&mut self) {
        self.position = 0;
        self.hits = 0;
        self.attempts = 0;
    }
}

/// Finite, stereo, envelope-shaped note; all state owned by this source.
/// Mixer gains are atomic and shared across voices (mixer faders act live).
struct NoteSource {
    rate: u32,
    frame: u64,
    total_frames: u64,
    note: f32,
    voice: usize,
    velocity: f32,
    settings: Option<MixSettings>,
    right: bool,
}
impl NoteSource {
    fn new(rate:u32, channel:usize, pitch:u8, velocity:f32, settings:MixSettings) -> Self {
        let seconds = [0.85, 0.58, 0.72, 1.3][channel];
        let frequency = 440. * 2f32.powf((f32::from(pitch)-69.)/12.);
        Self { rate: rate.max(8000), frame:0, total_frames:(rate as f32*seconds) as u64,
            note:frequency, voice:channel, velocity,settings:Some(settings),right:false }
    }
    fn click(rate:u32) -> Self {
        Self { rate:rate.max(8000), frame:0,total_frames:(rate as f32*0.045) as u64,
            note:880.,voice:4,velocity:0.35,settings:None,right:false }
    }
    fn sample(&self, frame:u64) -> f32 {
        let t=frame as f32/self.rate as f32;
        let progress=frame as f32/self.total_frames.max(1) as f32;
        let attack=(t/0.009).clamp(0.,1.);
        let release=((1.-progress)/0.18).clamp(0.,1.);
        let env=attack*release*release;
        let phase=TAU*self.note*t;
        let tone = match self.voice {
            0 => phase.sin()*0.76 + (2.*phase).sin()*0.17 + (3.*phase).sin()*0.07,
            1 => phase.sin()*0.77 + (2.*phase).sin()*0.23,
            2 => (phase.sin()*3.5).tanh()*0.45 + phase.sin()*0.35,
            3 => phase.sin()*0.65 + (2.01*phase).sin()*0.21 + (3.98*phase).sin()*0.14,
            _ => phase.sin(),
        };
        // Absence of clipping is important when 8+ polyphonic keys are held.
        tone*env*self.velocity*0.16
    }
}
impl Iterator for NoteSource {
    type Item=f32;
    fn next(&mut self)->Option<f32> {
        if self.frame >= self.total_frames { return None; }
        let sample=self.sample(self.frame);
        let left=!self.right;
        self.right = !self.right;
        if !left { self.frame+=1; }
        let (gain, pan, master) = match self.settings.as_ref() {
            Some(mix) => (
                mix.channels[self.voice].gain(),
                mix.channels[self.voice].pan(),
                f32::from_bits(mix.master.load(Ordering::Relaxed)),
            ),
            None => (0.5,0.,0.6),
        };
        // Equal-power pan: center channels each ~0.707.
        let angle=(pan+1.)*std::f32::consts::FRAC_PI_4;
        Some(sample*gain*master*if left {angle.cos()} else {angle.sin()})
    }
}
impl Source for NoteSource {
    fn current_span_len(&self)->Option<usize> {
        Some(((self.total_frames-self.frame)*2) as usize)
    }
    fn channels(&self)->ChannelCount { 2 }
    fn sample_rate(&self)->SampleRate { self.rate }
    fn total_duration(&self)->Option<Duration> {
        Some(Duration::from_secs_f32(self.total_frames as f32/self.rate as f32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pitch_names_and_frequencies_are_stable() {
        assert_eq!(pitch_label(60),"C4");
        assert_eq!(pitch_label(69),"A4");
        let note=NoteSource::new(DEFAULT_RATE,0,69,0.8,MixSettings::new());
        assert!((note.note-440.).abs()<0.01);
        assert_eq!(note.channels(),2);
    }
    #[test]
    fn practice_scale_advances_only_correct_notes() {
        let mut p=Practice::new();
        assert_eq!(p.target(),60);
        assert!(!p.press(61));
        assert!(p.press(60));
        assert_eq!(p.target(),62);
        assert_eq!(p.score(),(1,2));
        p.reset();
        assert_eq!(p.score(),(0,0));
    }
    #[test]
    fn notes_are_finite_and_end_cleanly() {
        let mut note=NoteSource::new(DEFAULT_RATE,2,72,1.,MixSettings::new());
        let count=note.by_ref().map(|sample| {
            assert!(sample.is_finite());
            sample.abs()
        }).count();
        assert_eq!(count, (DEFAULT_RATE as f32*0.72) as usize*2);
        assert!(note.next().is_none());
    }
    #[test]
    fn channel_mute_is_live_and_no_shared_lock_is_held() {
        let mix=MixSettings::new();
        let mut n=NoteSource::new(DEFAULT_RATE,0,60,1.,mix.clone());
        for _ in 0..4000 { let _=n.next(); }
        mix.channels[0].muted.store(true,Ordering::Relaxed);
        assert_eq!(n.next(),Some(0.));
    }
}
