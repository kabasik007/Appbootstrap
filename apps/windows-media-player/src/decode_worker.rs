//! Off-callback decoder producer: disk I/O and Symphonia decoding live on a
//! worker thread; the Rodio output source consumes only prebuffered f32 samples.
//! Seeking recreates the worker at a new position from the AUDIO CONTROL thread.
use crate::pcm_ring::PcmRing;
use rodio::{ChannelCount, Decoder, SampleRate, Source};
use std::{
    fs::File,
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
            let result = (|| -> Result<(Decoder<File>, TrackInfo), String> {
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
}
impl Iterator for BufferedPcmSource {
    type Item = f32;
    #[inline]
    fn next(&mut self) -> Option<f32> {
        if let Some(sample) = self.ring.try_pop() {
            self.underflow_samples = 0;
            return Some(sample);
        }
        if self.ring.is_drained() || self.ring.is_canceled() { return None; }
        // Count underruns once per 256 silent samples, not at audio sample rate.
        self.underflow_samples += 1;
        if self.underflow_samples >= 256 {
            self.ring.note_underrun();
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
        };
        assert_eq!(source.next(), Some(0.0));
        ring.try_push(0.5).unwrap();
        assert_eq!(source.next(), Some(0.5));
        ring.mark_finished();
        assert_eq!(source.next(), None);
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
            };
        }
        assert!(ring.is_canceled());
    }
}
