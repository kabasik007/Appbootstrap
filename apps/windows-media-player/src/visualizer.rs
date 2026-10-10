//! Real frequency spectrum analysis, isolated from the audio output thread.
//!
//! The output source adds one sample per audio frame to a small bounded SPSC
//! ring (no wait/lock/allocation). A separate worker runs the Hann-windowed
//! FFT and publishes 32 logarithmically spaced frequency bands using atomics.
//! When visuals lag, we DROP telemetry instead of making playback wait.
use crate::pcm_ring::PcmRing;
use rustfft::{num_complex::Complex32, FftPlanner};
use std::{
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub const BANDS: usize = 32;
const FFT_SIZE: usize = 2048;
const HOP: usize = 1024;

/// UI reads snapshots without sharing locks with the sound device.
pub struct Spectrum {
    revision: AtomicU64,
    audible: AtomicBool,
    levels: [AtomicU32; BANDS],
}

impl Spectrum {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            revision: AtomicU64::new(0),
            audible: AtomicBool::new(false),
            levels: std::array::from_fn(|_| AtomicU32::new(0)),
        })
    }

    fn start_track(&self) -> u64 {
        // Increment generation to invalidate any late worker from a previous
        // seek/track. The current output remains unaffected by analyzer failure.
        let generation = self.revision.fetch_add(1, Ordering::AcqRel) + 1;
        for level in &self.levels {
            level.store(0, Ordering::Relaxed);
        }
        generation
    }

    fn publish(&self, generation: u64, levels: &[f32; BANDS]) {
        if self.revision.load(Ordering::Acquire) != generation {
            return;
        }
        for (slot, value) in self.levels.iter().zip(levels.iter()) {
            slot.store(value.clamp(0.0, 100.0).to_bits(), Ordering::Relaxed);
        }
    }

    /// Hide stale display frames when transport is paused, stopped or idle.
    /// Visual data remains lossy; no UI callbacks enter the audio mixer.
    pub fn set_audible(&self, playing: bool) {
        self.audible.store(playing, Ordering::Relaxed);
    }

    pub fn read(&self) -> [f32; BANDS] {
        if !self.audible.load(Ordering::Relaxed) {
            return [0.0; BANDS];
        }
        std::array::from_fn(|index| {
            let value = f32::from_bits(self.levels[index].load(Ordering::Relaxed));
            if value.is_finite() { value.clamp(0.0, 100.0) } else { 0.0 }
        })
    }
}

/// Lightweight producer handle, owned by BufferedPcmSource.
pub struct VisualTap {
    ring: Arc<PcmRing>,
    channels: u16,
    channel_index: u16,
}

impl VisualTap {
    #[inline]
    pub fn sample(&mut self, value: f32) {
        // One sample per interleaved audio frame (first channel only for v0.1).
        // The second channel and other audio channels still play normally.
        if self.channel_index == 0 {
            let _ = self.ring.try_push(value);
        }
        self.channel_index += 1;
        if self.channel_index >= self.channels {
            self.channel_index = 0;
        }
    }

    pub fn cancel(&self) { self.ring.cancel(); }
}

/// Called on the audio control thread during new track prebuffering.
/// A failed visualizer spawn must never prevent music playback.
pub fn attach(snapshot: Arc<Spectrum>, sample_rate: u32, channels: u16) -> Option<VisualTap> {
    let ring = PcmRing::new(8_192);
    let generation = snapshot.start_track();
    let consumer = Arc::clone(&ring);
    let worker = thread::Builder::new()
        .name("zillaplayer-fft-analysis".into())
        .spawn(move || analyze(consumer, snapshot, generation, sample_rate.max(8_000)));
    if worker.is_err() {
        ring.cancel();
        return None;
    }
    Some(VisualTap {
        ring, channels: channels.max(1), channel_index: 0,
    })
}

fn band_levels(spectrum: &[Complex32], sample_rate: u32) -> [f32; BANDS] {
    let nyquist_bin = FFT_SIZE / 2;
    let nyquist = sample_rate as f32 / 2.0;
    std::array::from_fn(|band| {
        // Exponential bands from ~32 Hz to 16 kHz. Clip above Nyquist.
        let low_hz = 32.0 * (500.0_f32).powf(band as f32 / BANDS as f32);
        let high_hz = 32.0 * (500.0_f32).powf((band + 1) as f32 / BANDS as f32);
        if low_hz >= nyquist { return 0.0; }
        let low = ((low_hz * FFT_SIZE as f32 / sample_rate as f32).floor() as usize)
            .clamp(1, nyquist_bin);
        let high = ((high_hz.min(nyquist) * FFT_SIZE as f32 / sample_rate as f32)
            .ceil() as usize)
            .clamp(low, nyquist_bin);
        let mut max_power = 0.0_f32;
        for bin in low..=high {
            max_power = max_power.max(spectrum[bin].norm_sqr());
        }
        // Hann coherent amplitude ~0.5, so multiply 4/N for peak amplitude.
        let amplitude = (max_power.sqrt() * 4.0 / FFT_SIZE as f32).max(1e-7);
        let db = 20.0 * amplitude.log10();
        ((db + 75.0) * (100.0 / 75.0)).clamp(0.0, 100.0)
    })
}

fn analyze(ring: Arc<PcmRing>, snapshot: Arc<Spectrum>, generation: u64, sample_rate: u32) {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mut windowed = vec![Complex32::new(0.0, 0.0); FFT_SIZE];
    let mut wave = [0.0_f32; FFT_SIZE];
    let hann: [f32; FFT_SIZE] = std::array::from_fn(|i| {
        0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos())
    });
    let mut cursor = 0usize;
    let mut buffered = 0usize;
    let mut since_fft = 0usize;
    let mut smooth = [0.0_f32; BANDS];
    loop {
        if ring.is_canceled() || snapshot.revision.load(Ordering::Acquire) != generation {
            break;
        }
        let Some(sample) = ring.try_pop() else {
            if ring.is_drained() { break; }
            thread::sleep(Duration::from_millis(6));
            continue;
        };
        wave[cursor] = sample;
        cursor = (cursor + 1) % FFT_SIZE;
        buffered = buffered.saturating_add(1).min(FFT_SIZE);
        since_fft += 1;
        if buffered < FFT_SIZE || since_fft < HOP {
            continue;
        }
        since_fft = 0;
        for i in 0..FFT_SIZE {
            windowed[i] = Complex32::new(wave[(cursor + i) % FFT_SIZE] * hann[i], 0.0);
        }
        fft.process(&mut windowed);
        let next = band_levels(&windowed, sample_rate);
        for (value, next_value) in smooth.iter_mut().zip(next.iter()) {
            let speed = if next_value > value { 0.65 } else { 0.23 };
            *value += speed * (next_value - *value);
        }
        snapshot.publish(generation, &smooth);
    }
    // Only the last worker clears the bars; obsolete workers never update them.
    if snapshot.revision.load(Ordering::Acquire) == generation {
        snapshot.publish(generation, &[0.0; BANDS]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fft_detects_sine_near_one_khz() {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let rate = 48_000_u32;
        let bin = 43usize; // 1007.8 Hz
        let mut wave: Vec<Complex32> = (0..FFT_SIZE).map(|i| {
            let value = (2.0 * PI * bin as f32 * i as f32 / FFT_SIZE as f32).sin() * 0.8;
            let hann = 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos());
            Complex32::new(value * hann, 0.0)
        }).collect();
        fft.process(&mut wave);
        let levels = band_levels(&wave, rate);
        let max_index = levels.iter().enumerate()
            .max_by(|a,b| a.1.partial_cmp(b.1).unwrap()).unwrap().0;
        let freq_low = 32.0 * 500_f32.powf(max_index as f32 / BANDS as f32);
        let freq_high = 32.0 * 500_f32.powf((max_index+1) as f32 / BANDS as f32);
        // Bands intentionally overlap in FFT-bin space: allow tolerance.
        assert!(freq_low < 1_400.0 && freq_high > 500.0);
        assert!(levels[max_index] > 50.0);
    }

    #[test]
    fn clip_and_silence_produce_finite_levels() {
        let zeros = vec![Complex32::new(0.0, 0.0); FFT_SIZE];
        assert!(band_levels(&zeros, 44_100).iter().all(|level| *level == 0.0));
        let loud = vec![Complex32::new(1e20, 1e20); FFT_SIZE];
        assert!(band_levels(&loud, 48_000).iter().all(|value| value.is_finite() && *value <= 100.));
    }

    #[test]
    fn old_generation_cannot_overwrite_active_display() {
        let spectrum = Spectrum::new();
        let old = spectrum.start_track();
        spectrum.set_audible(true);
        spectrum.publish(old, &[50.0; BANDS]);
        let new = spectrum.start_track();
        spectrum.publish(old, &[100.0; BANDS]);
        assert!(spectrum.read().iter().all(|v| *v == 0.0));
        spectrum.publish(new, &[42.0; BANDS]);
        assert!(spectrum.read().iter().all(|v| *v == 42.0));
    }

    #[test]
    fn paused_spectrum_hides_stale_levels() {
        let spectrum = Spectrum::new();
        let track = spectrum.start_track();
        spectrum.set_audible(true);
        spectrum.publish(track, &[70.0; BANDS]);
        assert!(spectrum.read().iter().all(|value| *value > 0.0));
        spectrum.set_audible(false);
        assert!(spectrum.read().iter().all(|value| *value == 0.0));
        spectrum.set_audible(true);
        assert!(spectrum.read().iter().all(|value| *value == 70.0));
    }

    #[test]
    fn visual_tap_drops_data_instead_of_blocking() {
        let ring = PcmRing::new(4096);
        let mut tap = VisualTap { ring: Arc::clone(&ring), channels: 2, channel_index: 0 };
        for _ in 0..30_000 { tap.sample(0.8); tap.sample(-0.2); }
        assert_eq!(ring.available(), ring.capacity());
        tap.cancel();
        assert!(ring.is_canceled());
    }
}
