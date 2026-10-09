//! 31-band graphic EQ + bass / treble / loudness with per-channel biquad state.
//! Audio samples are processed with no allocations, mutexes, logging or disk access.
//! Filter coefficients and gain are calculated on the CONTROL worker and published
//! through an atomic revisioned snapshot. The audio path only reads atomics.
use rodio::{ChannelCount, SampleRate, Source};
use std::{f32::consts::PI, sync::{Arc, atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering}}, time::Duration};

pub const BAND_HZ: [f32; 31] = [
    20., 25., 31.5, 40., 50., 63., 80., 100., 125., 160.,
    200., 250., 315., 400., 500., 630., 800., 1000., 1250., 1600.,
    2000., 2500., 3150., 4000., 5000., 6300., 8000., 10000., 12500.,
    16000., 20000.,
];
const FILTER_COUNT: usize = 35; // 31 graphic + bass + treble + loudness low/high

fn atomic_float(value: f32) -> AtomicU32 { AtomicU32::new(value.to_bits()) }

/// Control worker is the sole writer; audio callback is the reader.
pub struct EqControls {
    bands: [AtomicU32; 31],
    preamp_db: AtomicU32,
    bass_db: AtomicU32,
    treble_db: AtomicU32,
    loudness: AtomicBool,
    enabled: AtomicBool,
    sample_rate: AtomicU32,
    revision: AtomicU64,
    // One frame contains five floating point coefficients per filter.
    prepared: [AtomicU32; FILTER_COUNT * 5],
    prepared_gain: AtomicU32,
}
impl EqControls {
    pub fn new() -> Arc<Self> {
        let controls = Arc::new(Self {
            bands: std::array::from_fn(|_| atomic_float(0.)),
            preamp_db: atomic_float(-6.),
            bass_db: atomic_float(0.),
            treble_db: atomic_float(0.),
            loudness: AtomicBool::new(false),
            enabled: AtomicBool::new(true),
            sample_rate: AtomicU32::new(44_100),
            revision: AtomicU64::new(0),
            prepared: std::array::from_fn(|i| atomic_float(if i % 5 == 0 { 1. } else { 0. })),
            prepared_gain: atomic_float(1.),
        });
        controls.prepare();
        controls
    }
    pub fn set_sample_rate(&self, rate: u32) {
        self.sample_rate.store(rate.max(8_000), Ordering::SeqCst);
        self.prepare();
    }
    pub fn set_band(&self, band: usize, db: f32) {
        if band < 31 && db.is_finite() {
            self.bands[band].store(db.clamp(-12., 12.).to_bits(), Ordering::SeqCst);
            self.prepare();
        }
    }
    pub fn set_preamp(&self, db: f32) {
        if db.is_finite() {
            self.preamp_db.store(db.clamp(-18., 6.).to_bits(), Ordering::SeqCst);
            self.prepare();
        }
    }
    pub fn set_bass(&self, db: f32) {
        if db.is_finite() {
            self.bass_db.store(db.clamp(-12., 12.).to_bits(), Ordering::SeqCst);
            self.prepare();
        }
    }
    pub fn set_treble(&self, db: f32) {
        if db.is_finite() {
            self.treble_db.store(db.clamp(-12., 12.).to_bits(), Ordering::SeqCst);
            self.prepare();
        }
    }
    pub fn set_loudness(&self, enabled: bool) {
        self.loudness.store(enabled, Ordering::SeqCst);
        self.prepare();
    }
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
        self.prepare();
    }
    fn prepare(&self) {
        // Never called by EqProcessor::process(). Only call on the command
        // thread or once during source construction.
        let rate = self.sample_rate.load(Ordering::SeqCst) as f32;
        let on = self.enabled.load(Ordering::SeqCst);
        let mut settings = [0_f32; FILTER_COUNT];
        if on {
            for (i, band) in self.bands.iter().enumerate() {
                settings[i] = f32::from_bits(band.load(Ordering::SeqCst));
            }
            settings[31] = f32::from_bits(self.bass_db.load(Ordering::SeqCst));
            settings[32] = f32::from_bits(self.treble_db.load(Ordering::SeqCst));
            if self.loudness.load(Ordering::SeqCst) {
                settings[33] = 3.;
                settings[34] = 1.5;
            }
        }
        let gain = if on {
            10_f32.powf(f32::from_bits(self.preamp_db.load(Ordering::SeqCst)) / 20.)
        } else { 1. };
        let coeffs: [Coeff; FILTER_COUNT] = std::array::from_fn(|idx| {
            let (hz, q, mode) = if idx < 31 {
                (BAND_HZ[idx], 4.3, Mode::Peak)
            } else if idx == 31 || idx == 33 {
                (95., 0.707, Mode::LowShelf)
            } else {
                (7200., 0.707, Mode::HighShelf)
            };
            coefficients(rate, hz, settings[idx], q, mode)
        });
        // A single writer publishes an odd revision while updating. Readers
        // skip the update if either revision changes or is odd. SeqCst avoids
        // using an incoherent mix of old/new coefficient fields.
        self.revision.fetch_add(1, Ordering::SeqCst);
        for (index, coeff) in coeffs.iter().enumerate() {
            let base = 5 * index;
            for (offset, field) in [coeff.b0,coeff.b1,coeff.b2,coeff.a1,coeff.a2]
                .into_iter().enumerate() {
                self.prepared[base + offset].store(field.to_bits(), Ordering::SeqCst);
            }
        }
        self.prepared_gain.store(gain.to_bits(), Ordering::SeqCst);
        self.revision.fetch_add(1, Ordering::SeqCst);
    }
}
#[derive(Clone, Copy)]
struct Coeff { b0: f32, b1: f32, b2: f32, a1: f32, a2: f32 }
impl Default for Coeff {
    fn default() -> Self { Self { b0:1., b1:0., b2:0., a1:0., a2:0. } }
}
#[derive(Clone, Copy, Default)]
struct Biquad { current: Coeff, target: Coeff, z1: f32, z2: f32 }
impl Biquad {
    fn process(&mut self, input: f32) -> f32 {
        // One-pole smoothing of coefficients avoids zipper noise on EQ movement.
        const STEP: f32 = 0.0025;
        self.current.b0 += STEP*(self.target.b0-self.current.b0);
        self.current.b1 += STEP*(self.target.b1-self.current.b1);
        self.current.b2 += STEP*(self.target.b2-self.current.b2);
        self.current.a1 += STEP*(self.target.a1-self.current.a1);
        self.current.a2 += STEP*(self.target.a2-self.current.a2);
        let y = self.current.b0*input + self.z1;
        self.z1 = self.current.b1*input - self.current.a1*y + self.z2;
        self.z2 = self.current.b2*input - self.current.a2*y;
        // Prevent denormal subnormal CPU spikes.
        if self.z1.abs() < 1e-19 { self.z1 = 0.; }
        if self.z2.abs() < 1e-19 { self.z2 = 0.; }
        if y.is_finite() { y } else { *self = Self::default(); 0. }
    }
    fn idle(&self) -> bool {
        (self.target.b0-1.).abs() < 1e-6 &&
        (self.current.b0-1.).abs() < 1e-5 &&
        self.z1.abs() < 1e-6 && self.z2.abs() < 1e-6
    }
}
#[derive(Clone, Copy)]
enum Mode { Peak, LowShelf, HighShelf }
fn coefficients(rate: f32, center: f32, gain_db: f32, q: f32, mode: Mode) -> Coeff {
    if gain_db.abs() < 0.0001 { return Coeff::default(); }
    let frequency = center.clamp(10., rate*0.45);
    let w = 2.*PI*frequency/rate;
    let (sin, cos) = w.sin_cos();
    let a = 10_f32.powf(gain_db/40.);
    let alpha = sin/(2.*q.max(0.1));
    let (b0,b1,b2,a0,a1,a2) = match mode {
        Mode::Peak => (1.+alpha*a, -2.*cos, 1.-alpha*a,
                       1.+alpha/a, -2.*cos, 1.-alpha/a),
        Mode::LowShelf | Mode::HighShelf => {
            // RBJ shelf with slope 1.0.
            let beta = 2.*a.sqrt()*alpha;
            if matches!(mode,Mode::LowShelf) {
                (a*((a+1.)-(a-1.)*cos+beta),
                 2.*a*((a-1.)-(a+1.)*cos),
                 a*((a+1.)-(a-1.)*cos-beta),
                 (a+1.)+(a-1.)*cos+beta,
                 -2.*((a-1.)+(a+1.)*cos),
                 (a+1.)+(a-1.)*cos-beta)
            } else {
                (a*((a+1.)+(a-1.)*cos+beta),
                 -2.*a*((a-1.)+(a+1.)*cos),
                 a*((a+1.)+(a-1.)*cos-beta),
                 (a+1.)-(a-1.)*cos+beta,
                 2.*((a-1.)-(a+1.)*cos),
                 (a+1.)-(a-1.)*cos-beta)
            }
        }
    };
    if a0.abs() < 1e-9 { return Coeff::default(); }
    Coeff { b0:b0/a0,b1:b1/a0,b2:b2/a0,a1:a1/a0,a2:a2/a0 }
}
pub struct EqProcessor {
    controls: Arc<EqControls>,
    filters: Vec<[Biquad; FILTER_COUNT]>,
    channel: usize,
    frames_to_refresh: usize,
    applied_revision: u64,
    amp: f32,
    target_amp: f32,
}
impl EqProcessor {
    pub fn new(rate: u32, channels: usize, controls: Arc<EqControls>) -> Self {
        // This constructor is called on the AUDIO CONTROL thread.
        controls.set_sample_rate(rate);
        let channels = channels.clamp(1, 16);
        Self {
            controls, filters: vec![[Biquad::default(); FILTER_COUNT]; channels],
            channel: 0, frames_to_refresh: 0, applied_revision: u64::MAX,
            amp: 1., target_amp: 1.,
        }
    }
    fn refresh(&mut self) {
        let before = self.controls.revision.load(Ordering::SeqCst);
        if before == self.applied_revision || before & 1 != 0 { return; }
        let mut copy = [Coeff::default(); FILTER_COUNT];
        for (index, coeff) in copy.iter_mut().enumerate() {
            let b = index * 5;
            let read = |offset: usize| f32::from_bits(
                self.controls.prepared[b + offset].load(Ordering::SeqCst)
            );
            *coeff = Coeff {
                b0: read(0), b1: read(1), b2: read(2),
                a1: read(3), a2: read(4),
            };
        }
        let gain = f32::from_bits(self.controls.prepared_gain.load(Ordering::SeqCst));
        let after = self.controls.revision.load(Ordering::SeqCst);
        if before != after || after & 1 != 0 { return; }
        // Only stable, complete coefficient snapshots reach the filters.
        for filters in &mut self.filters {
            for (filter, coeff) in filters.iter_mut().zip(copy.iter()) {
                filter.target = *coeff;
            }
        }
        self.target_amp = gain;
        self.applied_revision = after;
    }
    pub fn process(&mut self, sample: f32) -> f32 {
        if self.channel == 0 {
            if self.frames_to_refresh == 0 {
                self.refresh();
                self.frames_to_refresh = 128;
            }
            self.frames_to_refresh -= 1;
            self.amp += 0.0009 * (self.target_amp - self.amp);
        }
        let chan = self.channel;
        self.channel = (self.channel + 1) % self.filters.len();
        let mut value = sample;
        for filter in &mut self.filters[chan] {
            if !filter.idle() { value = filter.process(value); }
        }
        (value * self.amp).clamp(-1., 1.)
    }
    pub fn reset(&mut self) {
        for filters in &mut self.filters {
            for filter in filters {
                filter.z1 = 0.; filter.z2 = 0.;
            }
        }
        self.channel = 0;
    }
}
pub struct EqSource<S:Source<Item=f32>> { source:S, processor:EqProcessor }
impl<S:Source<Item=f32>> EqSource<S> {
    pub fn new(source:S, controls:Arc<EqControls>) -> Self {
        let rate=source.sample_rate().get();
        let channels=source.channels().get() as usize;
        Self {source,processor:EqProcessor::new(rate,channels,controls)}
    }
}
impl<S:Source<Item=f32>> Iterator for EqSource<S> {
    type Item=f32;
    fn next(&mut self)->Option<f32> {
        self.source.next().map(|sample|self.processor.process(sample))
    }
}
impl<S:Source<Item=f32>> Source for EqSource<S> {
    fn current_span_len(&self)->Option<usize> { self.source.current_span_len() }
    fn channels(&self)->ChannelCount { self.source.channels() }
    fn sample_rate(&self)->SampleRate { self.source.sample_rate() }
    fn total_duration(&self)->Option<Duration> { self.source.total_duration() }
    fn try_seek(&mut self,pos:Duration)->Result<(),rodio::source::SeekError> {
        self.source.try_seek(pos)?;
        self.processor.reset();
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn no_nan_and_silence_remains_silent() {
        let mut eq=EqProcessor::new(44_100,2,EqControls::new());
        for _ in 0..2048 { assert_eq!(eq.process(0.),0.); }
    }
    #[test] fn band_changes_audio_and_bypass_recovers() {
        let controls=EqControls::new();
        let mut eq=EqProcessor::new(48_000,2,controls.clone());
        controls.set_band(16,9.);
        controls.set_bass(6.);
        let mut sum=0.;
        for n in 0..30_000 {
            let sample=((n as f32)*0.03).sin()*0.1;
            let output=eq.process(sample);
            assert!(output.is_finite());
            sum+=(output-sample).abs();
        }
        assert!(sum>5.);
        controls.set_enabled(false);
        for _ in 0..1000 { assert!(eq.process(0.).is_finite()); }
    }
    #[test] fn coefficients_are_finite() {
        for rate in [8_000.,44_100.,96_000.] {
            for f in BAND_HZ {
                for db in [-12.,6.,12.] {
                    let c=coefficients(rate,f,db,4.3,Mode::Peak);
                    assert!(c.b0.is_finite()&&c.a2.is_finite());
                }
            }
        }
    }
}
