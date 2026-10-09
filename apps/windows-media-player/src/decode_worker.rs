//! Off-callback decoder producer: disk I/O and Symphonia decoding live on a
//! worker thread; the Rodio output source consumes only prebuffered f32 samples.
//! Seeking recreates the worker at a new position from the AUDIO CONTROL thread.
use crate::pcm_ring::PcmRing;
use rodio::{ChannelCount, Decoder, SampleRate, Source};
use std::{
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

const MAX_OPEN_WAIT: Duration = Duration::from_secs(4);
const PREBUFFER_WAIT: Duration = Duration::from_millis(350);

pub struct PreparedAudio {
    pub source: BufferedPcmSource,
    pub cancel: Arc<PcmRing>,
    pub duration: Option<Duration>,
}

struct TrackInfo {
    ring: Arc<PcmRing>,
    channels: ChannelCount,
    sample_rate: SampleRate,
    duration: Option<Duration>,
}

struct FinishOnDrop(Arc<PcmRing>);
impl Drop for FinishOnDrop {
    fn drop(&mut self) { self.0.mark_finished(); }
}

/// Blocking function called ONLY on the audio control thread, never Slint.
/// Timeouts keep the command worker from waiting indefinitely on slow disks.
pub fn prepare(path: PathBuf, position: Duration) -> Result<PreparedAudio, String> {
    let (tx, rx) = mpsc::sync_channel::<Result<TrackInfo, String>>(1);
    thread::Builder::new()
        .name("zillaplayer-decode-worker".into())
        .spawn(move || {
            let result = (|| -> Result<(Decoder<BufReader<File>>, TrackInfo), String> {
                let file = File::open(&path)
                    .map_err(|e| format!("Cannot read audio file: {e}"))?;
                let mut decoder = Decoder::try_from(file)
                    .map_err(|e| format!("Unsupported or damaged media: {e}"))?;
                let duration = decoder.total_duration();
                if !position.is_zero() {
                    decoder.try_seek(position)
                        .map_err(|e| format!("Cannot seek in this file: {e}"))?;
                }
                let channels = decoder.channels();
                let sample_rate = decoder.sample_rate();
                let requested = (sample_rate.get() as usize)
                    .saturating_mul(channels.get() as usize).saturating_mul(2);
                let ring = PcmRing::new(requested);
                Ok((decoder, TrackInfo { ring, channels, sample_rate, duration }))
            })();

            let (decoder, info) = match result {
                Ok(value) => value,
                Err(reason) => { let _ = tx.send(Err(reason)); return; }
            };
            let ring = Arc::clone(&info.ring);
            if tx.send(Ok(info)).is_err() { return; }
            let _finish = FinishOnDrop(Arc::clone(&ring));
            for sample in decoder {
                // Backpressure and decoding happen only on this worker.
                // Cancellation is checked even if the buffer is full.
                if ring.is_canceled() { break; }
                let mut candidate = sample;
                loop {
                    if ring.is_canceled() { return; }
                    match ring.try_push(candidate) {
                        Ok(()) => break,
                        Err(sample) => {
                            candidate = sample;
                            thread::sleep(Duration::from_millis(2));
                        }
                    }
                }
            }
        })
        .map_err(|e| format!("Cannot start decoder thread: {e}"))?;

    let info = rx.recv_timeout(MAX_OPEN_WAIT)
        .map_err(|e| format!("Audio decoder timed out: {e}"))??;
    // Prime the buffer (about 100 ms) before attaching source to the mixer.
    // If slow disks take longer, the audio callback emits bounded silence,
    // never waits for storage/decoder locks.
    let prebuffer = (info.sample_rate.get() as usize)
        .saturating_mul(info.channels.get() as usize) / 10;
    let deadline = Instant::now() + PREBUFFER_WAIT;
    while info.ring.available() < prebuffer &&
        !info.ring.is_drained() &&
        Instant::now() < deadline
    {
        thread::sleep(Duration::from_millis(3));
    }

    let cancel = Arc::clone(&info.ring);
    let duration = info.duration;
    Ok(PreparedAudio {
        source: BufferedPcmSource {
            ring: info.ring,
            channels: info.channels,
            sample_rate: info.sample_rate,
            duration,
            underflow_samples: 0,
            analysis_samples: 0,
            peak_window: 0.0,
        },
        cancel,
        duration,
    })
}

/// Only this iterator is consumed by the Rodio mixer. No decoding, file reads,
/// heap allocation or thread blocking occurs in next(). A temporary shortage
/// emits silence; true EOF is emitted only after producer completion + drain.
pub struct BufferedPcmSource {
    ring: Arc<PcmRing>,
    channels: ChannelCount,
    sample_rate: SampleRate,
    duration: Option<Duration>,
    underflow_samples: u32,
    analysis_samples: u16,
    peak_window: f32,
}
impl Iterator for BufferedPcmSource {
    type Item = f32;
    #[inline]
    fn next(&mut self) -> Option<f32> {
        if let Some(sample) = self.ring.try_pop() {
            self.underflow_samples = 0;
            self.peak_window = self.peak_window.max(sample.abs());
            self.analysis_samples += 1;
            if self.analysis_samples >= 1024 {
                self.ring.set_peak_level(self.peak_window);
                self.analysis_samples = 0;
                self.peak_window = 0.0;
            }
            return Some(sample);
        }
        if self.ring.is_drained() || self.ring.is_canceled() {
            self.ring.set_peak_level(0.0);
            return None;
        }
        // Count underruns once per 256 silent samples, not at audio sample rate.
        self.underflow_samples += 1;
        if self.underflow_samples >= 256 {
            self.ring.note_underrun();
            self.ring.set_peak_level(0.0);
            self.underflow_samples = 0;
        }
        Some(0.0)
    }
}
impl Source for BufferedPcmSource {
    fn current_span_len(&self) -> Option<usize> { None }
    fn channels(&self) -> ChannelCount { self.channels }
    fn sample_rate(&self) -> SampleRate { self.sample_rate }
    fn total_duration(&self) -> Option<Duration> { self.duration }
}
impl Drop for BufferedPcmSource {
    fn drop(&mut self) { self.ring.cancel(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::{NonZeroU16, NonZeroU32};

    #[test]
    fn buffer_underrun_is_silence_until_true_eof() {
        let ring = PcmRing::new(4096);
        let mut source = BufferedPcmSource {
            ring: Arc::clone(&ring),
            channels: NonZeroU16::new(2).unwrap(),
            sample_rate: NonZeroU32::new(44_100).unwrap(),
            duration: Some(Duration::from_secs(10)),
            underflow_samples: 0,
            analysis_samples: 0,
            peak_window: 0.0,
        };
        assert_eq!(source.next(), Some(0.0));
        ring.try_push(0.5).unwrap();
        assert_eq!(source.next(), Some(0.5));
        ring.mark_finished();
        assert_eq!(source.next(), None);
    }

    #[test]
    fn generated_wav_decodes_on_worker_and_produces_pcm() {
        use std::{fs, time::{SystemTime, UNIX_EPOCH}};
        // Generate a tiny redistributable WAV in memory; no test music required.
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!(
            "zillaplayer-pcm-{}-{stamp}.wav", std::process::id()
        ));
        let frames: usize = 8_820; // 200 ms @ 44.1 kHz, mono, 16-bit PCM.
        let data_len = (frames * 2) as u32;
        let mut wav = Vec::with_capacity(44 + data_len as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1_u16.to_le_bytes()); // 1 channel
        wav.extend_from_slice(&44_100_u32.to_le_bytes());
        wav.extend_from_slice(&88_200_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        for index in 0..frames {
            let sample: i16 = if index % 4 < 2 { 12_000 } else { -12_000 };
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        fs::write(&path, wav).unwrap();

        let mut prepared = prepare(path.clone(), Duration::ZERO)
            .expect("generated WAV should be accepted by the decoder");
        assert_eq!(prepared.source.channels.get(), 1);
        assert_eq!(prepared.source.sample_rate.get(), 44_100);
        let mut heard_signal = false;
        for _ in 0..3_000 {
            if let Some(sample) = prepared.source.next() {
                if sample.abs() > 0.1 { heard_signal = true; break; }
            } else { break; }
        }
        assert!(heard_signal, "decoded PCM must contain an audible sample");
        prepared.cancel.cancel();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn level_meter_reads_real_pcm_samples_without_waiting() {
        let ring = PcmRing::new(4096);
        for _ in 0..1024 { ring.try_push(0.65).unwrap(); }
        let mut source = BufferedPcmSource {
            ring: Arc::clone(&ring),
            channels: NonZeroU16::new(2).unwrap(),
            sample_rate: NonZeroU32::new(44_100).unwrap(),
            duration: None,
            underflow_samples: 0,
            analysis_samples: 0,
            peak_window: 0.0,
        };
        for _ in 0..1024 { assert_eq!(source.next(), Some(0.65)); }
        assert!((ring.peak_percent() - 65.).abs() < 0.001);
    }

    #[test]
    fn consumer_drop_signals_decoder_cancellation() {
        let ring = PcmRing::new(4096);
        {
            let _source = BufferedPcmSource {
                ring: Arc::clone(&ring),
                channels: NonZeroU16::new(2).unwrap(),
                sample_rate: NonZeroU32::new(48_000).unwrap(),
                duration: None,
                underflow_samples: 0,
                analysis_samples: 0,
                peak_window: 0.0,
            };
        }
        assert!(ring.is_canceled());
    }
}
