//! UI-independent playback state and validation helpers.
//! This module contains no output-device code and is unit-testable.
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Idle,
    Playing,
    Paused,
    Stopped,
    Error,
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub title: String,
    pub transport: Transport,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub volume: f32,
    pub detail: String,
    pub has_track: bool,
    /// Counts 256-sample starvation windows (not exact hardware callback drops).
    pub buffer_starvations: u64,
    pub audio_level_percent: f32,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            title: "No track loaded".into(),
            transport: Transport::Idle,
            position: Duration::ZERO,
            duration: None,
            volume: 0.8,
            detail: "Open a local MP3, FLAC, WAV, OGG or M4A file".into(),
            has_track: false,
            buffer_starvations: 0,
            audio_level_percent: 0.0,
        }
    }
}

impl PlaybackState {
    pub fn progress_percent(&self) -> f32 {
        match self.duration {
            Some(total) if !total.is_zero() => {
                (100.0 * self.position.as_secs_f32() / total.as_secs_f32())
                    .clamp(0.0, 100.0)
            }
            _ => 0.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self.transport {
            Transport::Idle => "Idle",
            Transport::Playing => "Playing",
            Transport::Paused => "Paused",
            Transport::Stopped => "Stopped",
            Transport::Error => "Error",
        }
    }

    pub fn play_caption(&self) -> &'static str {
        if self.transport == Transport::Playing {
            "❚❚ Pause"
        } else {
            "▶ Play"
        }
    }
}

/// A zero-slope envelope at both ends for seek/pause/track changes.
pub fn fade_weight(t: f32) -> f32 {
    let t = if t.is_finite() { t.clamp(0.,1.) } else { 0. };
    (1.0 - (std::f32::consts::PI * t).cos()) * 0.5
}

pub fn normalise_volume(percent: f32) -> f32 {
    if percent.is_finite() {
        (percent / 100.0).clamp(0.0, 1.0)
    } else {
        0.8
    }
}

pub fn seek_position(percent: f32, duration: Option<Duration>) -> Option<Duration> {
    let total = duration?;
    if !percent.is_finite() || total.is_zero() {
        return None;
    }
    Some(total.mul_f32((percent / 100.0).clamp(0.0, 1.0)))
}

pub fn media_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|part| part.to_str())
        .filter(|title| !title.trim().is_empty())
        .unwrap_or("Unknown track")
        .to_string()
}

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hrs = seconds / 3600;
    let mins = (seconds / 60) % 60;
    let secs = seconds % 60;
    if hrs > 0 {
        format!("{hrs}:{mins:02}:{secs:02}")
    } else {
        format!("{mins:02}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn fade_has_soft_edges_and_is_monotonic() {
        assert!(fade_weight(0.0).abs()<1e-6);
        assert!((fade_weight(1.0)-1.).abs()<1e-6);
        let mut before = -1.;
        for i in 0..=100 {
            let value = fade_weight(i as f32 / 100.);
            assert!(value >= before);
            before = value;
        }
        assert_eq!(fade_weight(f32::NAN), 0.);
    }

    #[test]
    fn progress_is_bounded() {
        let mut state = PlaybackState::default();
        assert_eq!(state.progress_percent(), 0.0);
        state.duration = Some(Duration::from_secs(10));
        state.position = Duration::from_secs(20);
        assert_eq!(state.progress_percent(), 100.0);
    }

    #[test]
    fn seek_clamps_and_rejects_unknown_duration() {
        let dur = Some(Duration::from_secs(200));
        assert_eq!(seek_position(50.0, dur), Some(Duration::from_secs(100)));
        assert_eq!(seek_position(-1.0, dur), Some(Duration::ZERO));
        assert_eq!(seek_position(1000.0, dur), dur);
        assert_eq!(seek_position(f32::NAN, dur), None);
        assert_eq!(seek_position(40.0, None), None);
    }

    #[test]
    fn volume_is_always_finite_and_safe() {
        assert_eq!(normalise_volume(150.0), 1.0);
        assert_eq!(normalise_volume(-10.0), 0.0);
        assert_eq!(normalise_volume(f32::NAN), 0.8);
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(format_duration(Duration::from_secs(71)), "01:11");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1:01:01");
        assert_eq!(media_title(Path::new("C:/Music/song.mp3")), "song");
    }
}
