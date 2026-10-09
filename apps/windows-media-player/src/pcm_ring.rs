//! Bounded lock-free single-producer / single-consumer f32 sample queue.
//! No heap allocations, mutexes, waits, decoding or filesystem I/O in pop().
//!
//! Exactly one decode worker calls try_push(), and exactly one Rodio source
//! calls try_pop(). Indices publish samples using Release/Acquire ordering.
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    Arc,
};

pub struct PcmRing {
    samples: Box<[AtomicU32]>,
    capacity: u64,
    reader: AtomicU64,
    writer: AtomicU64,
    finished: AtomicBool,
    canceled: AtomicBool,
    underruns: AtomicU64,
}

impl PcmRing {
    /// Buffer is bounded between 4,096 and 1,048,576 interleaved samples.
    pub fn new(requested_samples: usize) -> Arc<Self> {
        let capacity = requested_samples.clamp(4_096, 1_048_576).next_power_of_two();
        Arc::new(Self {
            samples: (0..capacity).map(|_| AtomicU32::new(0)).collect(),
            capacity: capacity as u64,
            reader: AtomicU64::new(0),
            writer: AtomicU64::new(0),
            finished: AtomicBool::new(false),
            canceled: AtomicBool::new(false),
            underruns: AtomicU64::new(0),
        })
    }

    #[inline]
    pub fn try_push(&self, sample: f32) -> Result<(), f32> {
        let write = self.writer.load(Ordering::Relaxed);
        if write.wrapping_sub(self.reader.load(Ordering::Acquire)) >= self.capacity {
            return Err(sample);
        }
        self.samples[(write % self.capacity) as usize]
            .store(sample.to_bits(), Ordering::Relaxed);
        self.writer.store(write.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    #[inline]
    pub fn try_pop(&self) -> Option<f32> {
        let read = self.reader.load(Ordering::Relaxed);
        if read == self.writer.load(Ordering::Acquire) {
            return None;
        }
        let sample = f32::from_bits(
            self.samples[(read % self.capacity) as usize].load(Ordering::Relaxed),
        );
        self.reader.store(read.wrapping_add(1), Ordering::Release);
        Some(sample)
    }

    pub fn available(&self) -> usize {
        let w = self.writer.load(Ordering::Acquire);
        let r = self.reader.load(Ordering::Acquire);
        w.wrapping_sub(r).min(self.capacity) as usize
    }

    pub fn capacity(&self) -> usize { self.capacity as usize }

    pub fn mark_finished(&self) { self.finished.store(true, Ordering::Release); }

    pub fn is_drained(&self) -> bool {
        self.finished.load(Ordering::Acquire) && self.available() == 0
    }

    pub fn cancel(&self) { self.canceled.store(true, Ordering::Release); }

    pub fn is_canceled(&self) -> bool { self.canceled.load(Ordering::Acquire) }

    pub fn note_underrun(&self) { self.underruns.fetch_add(1, Ordering::Relaxed); }

    pub fn underruns(&self) -> u64 { self.underruns.load(Ordering::Relaxed) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn fifo_and_bounds_are_correct() {
        let ring = PcmRing::new(5);
        assert_eq!(ring.capacity(), 4096);
        assert_eq!(ring.try_pop(), None);
        for i in 0..ring.capacity() {
            assert!(ring.try_push(i as f32).is_ok());
        }
        assert!(ring.try_push(-1.0).is_err());
        for i in 0..ring.capacity() {
            assert_eq!(ring.try_pop(), Some(i as f32));
        }
        assert_eq!(ring.try_pop(), None);
        ring.mark_finished();
        assert!(ring.is_drained());
    }

    #[test]
    fn sequence_integrity_across_threads_and_wraparound() {
        const N: usize = 45_000;
        let ring = PcmRing::new(4_096);
        let producer = Arc::clone(&ring);
        let worker = thread::spawn(move || {
            for i in 0..N {
                while producer.try_push(i as f32).is_err() {
                    thread::yield_now();
                }
            }
            producer.mark_finished();
        });
        for expected in 0..N {
            let sample = loop {
                if let Some(value) = ring.try_pop() { break value; }
                thread::yield_now();
            };
            assert_eq!(sample, expected as f32);
        }
        worker.join().unwrap();
        assert!(ring.is_drained());
    }

    #[test]
    fn empty_buffer_is_not_the_same_as_finished() {
        let ring = PcmRing::new(4096);
        assert_eq!(ring.try_pop(), None);
        assert!(!ring.is_drained());
        ring.note_underrun();
        assert_eq!(ring.underruns(), 1);
        ring.mark_finished();
        assert!(ring.is_drained());
    }
}
