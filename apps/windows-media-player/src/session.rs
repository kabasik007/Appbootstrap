//! Versioned local playback queue persistence.
//! All JSON/filesystem operations run on a dedicated worker, NEVER on Slint
//! or the Rodio output callback. This is an interim v1 session format;
//! the full searchable library will later migrate to SQLite.
use crate::presets::{self, EqPreset};
use std::{
    cell::RefCell,
    fs,
    io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

const MAX_TRACKS: usize = 50_000;
const MAX_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NamedPlaylist {
    pub name: String,
    pub tracks: Vec<PathBuf>,
}
#[derive(Debug, Clone)]
pub struct SavedSession {
    pub tracks: Vec<PathBuf>,
    pub selected: Option<usize>,
    pub playlists: Vec<NamedPlaylist>,
    pub presets: Vec<EqPreset>,
    pub last_eq: EqPreset,
}
impl Default for SavedSession {
    fn default() -> Self {
        Self {
            tracks: Vec::new(), selected: None, playlists: Vec::new(),
            presets: Vec::new(), last_eq: EqPreset::flat(),
        }
    }
}

impl SavedSession {
    pub fn sanitized(mut self) -> Self {
        self.tracks.truncate(MAX_TRACKS);
        if self.selected.is_some_and(|i| i >= self.tracks.len()) {
            self.selected = None;
        }
        self.playlists.truncate(64);
        self.playlists.retain_mut(|playlist| {
            playlist.name = playlist.name.trim().chars().take(64).collect();
            playlist.tracks.truncate(MAX_TRACKS);
            !playlist.name.is_empty()
        });
        self.presets.truncate(24);
        self.presets = self.presets.into_iter().map(EqPreset::sanitize).collect();
        self.last_eq = self.last_eq.sanitize();
        self
    }
}

enum Message {
    Save(SavedSession),
    Shutdown,
}

pub struct SessionStore {
    updates: Receiver<Result<SavedSession, String>>,
    sender: Sender<Message>,
    worker: RefCell<Option<JoinHandle<()>>>,
}

impl SessionStore {
    pub fn start() -> io::Result<Self> {
        let path = default_path();
        let (sender, commands) = mpsc::channel();
        let (loaded, updates) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("zillaplayer-session-io".into())
            .spawn(move || {
                let result = read_from(&path).map_err(|error| error.to_string());
                let _ = loaded.send(result);
                while let Ok(message) = commands.recv() {
                    match message {
                        Message::Save(mut snapshot) => {
                            // Prefer newest pending state over stale queued saves.
                            let mut shutdown = false;
                            while let Ok(next) = commands.try_recv() {
                                match next {
                                    Message::Save(newer) => snapshot = newer,
                                    Message::Shutdown => { shutdown = true; break; }
                                }
                            }
                            if let Err(error) = write_to(&path, &snapshot) {
                                eprintln!("ZillaPlayer: session save failed: {error}");
                            }
                            if shutdown { break; }
                        }
                        Message::Shutdown => break,
                    }
                }
            })?;
        Ok(Self { updates, sender, worker: RefCell::new(Some(worker)) })
    }

    pub fn poll_loaded(&self) -> Option<Result<SavedSession, String>> {
        self.updates.try_recv().ok()
    }

    pub fn enqueue_save(&self, session: SavedSession) {
        let _ = self.sender.send(Message::Save(session.sanitized()));
    }

    pub fn shutdown(&self, latest: SavedSession) {
        self.enqueue_save(latest);
        let _ = self.sender.send(Message::Shutdown);
        if let Some(worker) = self.worker.borrow_mut().take() { let _ = worker.join(); }
    }
}

fn default_path() -> PathBuf {
    let parent = std::env::var_os("APPDATA").map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| {
            PathBuf::from(home).join(".local").join("share")
        }))
        .unwrap_or_else(std::env::temp_dir);
    parent.join("ZillaPlayer").join("session-v1.json")
}

fn read_from(path: &Path) -> io::Result<SavedSession> {
    let backup = path.with_extension("json.bak");
    match read_one(path) {
        Ok(Some(state)) => Ok(state),
        Ok(None) => Ok(read_one(&backup)?.unwrap_or_default()),
        Err(first) => match read_one(&backup) {
            Ok(Some(state)) => Ok(state),
            _ => Err(first),
        },
    }
}

fn read_one(path: &Path) -> io::Result<Option<SavedSession>> {
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if meta.len() > MAX_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Session file too large"));
    }
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)
        .map_err(io::Error::other)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(1) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Unknown session version"));
    }
    let array = value.get("tracks").and_then(|v| v.as_array())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing tracks"))?;
    if array.len() > MAX_TRACKS {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Too many tracks"));
    }
    let mut tracks = Vec::with_capacity(array.len());
    for item in array {
        let path = item.as_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid track path"))?;
        tracks.push(PathBuf::from(path));
    }
    let selected = value.get("selected").and_then(|v| v.as_u64())
        .and_then(|index| usize::try_from(index).ok());
    // Fields introduced after session v1 are OPTIONAL, so an existing
    // installed alpha keeps its saved queue during the upgrade.
    let playlists = value.get("playlists").and_then(|v| v.as_array())
        .map(|entries| entries.iter().take(64).filter_map(|entry| {
            let name = entry.get("name")?.as_str()?.to_string();
            let items = entry.get("tracks")?.as_array()?;
            let tracks = items.iter().take(MAX_TRACKS)
                .map(|item| item.as_str().map(PathBuf::from))
                .collect::<Option<Vec<_>>>()?;
            Some(NamedPlaylist { name, tracks })
        }).collect()).unwrap_or_default();
    let presets = value.get("presets").and_then(|v| v.as_array())
        .map(|entries| entries.iter().take(24).filter_map(presets::from_json).collect())
        .unwrap_or_default();
    let last_eq = value.get("last_eq").and_then(presets::from_json)
        .unwrap_or_else(EqPreset::flat);
    Ok(Some(SavedSession { tracks, selected, playlists, presets, last_eq }.sanitized()))
}

fn write_to(path: &Path, snapshot: &SavedSession) -> io::Result<()> {
    let snapshot = snapshot.clone().sanitized();
    let parent = path.parent().ok_or_else(|| io::Error::other("No session directory"))?;
    fs::create_dir_all(parent)?;
    let value = serde_json::json!({
        "version": 1,
        "tracks": snapshot.tracks.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
        "selected": snapshot.selected,
        "playlists": snapshot.playlists.iter().map(|p| serde_json::json!({
            "name": p.name, "tracks": p.tracks.iter().map(|v| v.to_string_lossy().to_string()).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "presets": snapshot.presets.iter().map(presets::to_json).collect::<Vec<_>>(),
        "last_eq": presets::to_json(&snapshot.last_eq),
    });
    let payload = serde_json::to_vec(&value).map_err(io::Error::other)?;
    if payload.len() as u64 > MAX_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Session exceeds 16 MiB"));
    }
    let temp = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    fs::write(&temp, payload)?;
    if path.exists() {
        // Keep the last valid backup. If the primary was corrupted by a
        // previous crash, never replace a working backup with broken JSON.
        match read_one(path) {
            Ok(Some(_)) => {
                if backup.exists() { fs::remove_file(&backup)?; }
                fs::rename(path, &backup)?;
            }
            _ => fs::remove_file(path)?,
        }
    }
    if let Err(error) = fs::rename(&temp, path) {
        if backup.exists() { let _ = fs::rename(&backup, path); }
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    fn temp_path() -> PathBuf {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("zillaplayer-session-{}-{timestamp}.json",std::process::id()))
    }

    #[test]
    fn unicode_and_selection_round_trip() {
        let path = temp_path();
        let before = SavedSession {
            tracks: vec![PathBuf::from("C:/Музика/Трек №1.flac"), PathBuf::from("D:/play.mp3")],
            selected: Some(1),
            ..SavedSession::default()

        };
        write_to(&path, &before).unwrap();
        let read = read_from(&path).unwrap();
        assert_eq!(read.tracks, before.tracks);
        assert_eq!(read.selected, before.selected);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn corruption_falls_back_to_backup() {
        let path = temp_path();
        let initial = SavedSession {
            tracks: vec![PathBuf::from("ok.wav")], selected: Some(0),
            ..SavedSession::default()

        };
        write_to(&path, &initial).unwrap();
        write_to(&path, &SavedSession {
            tracks: vec![PathBuf::from("new.wav")], selected: Some(0),
            ..SavedSession::default()

        }).unwrap();
        fs::write(&path, b"corrupt-json").unwrap();
        let recovered = read_from(&path).unwrap();
        assert_eq!(recovered.tracks, initial.tracks);
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.bak")).unwrap();
    }

    #[test]
    fn saving_over_corrupt_primary_preserves_valid_backup() {
        let path = temp_path();
        let first = SavedSession {
            tracks: vec![PathBuf::from("first.flac")], selected: Some(0),
            ..SavedSession::default()

        };
        write_to(&path, &first).unwrap();
        write_to(&path, &SavedSession {
            tracks: vec![PathBuf::from("second.mp3")], selected: Some(0),
            ..SavedSession::default()

        }).unwrap();
        fs::write(&path, b"{garbled").unwrap();
        write_to(&path, &SavedSession {
            tracks: vec![PathBuf::from("third.wav")], selected: Some(0),
            ..SavedSession::default()

        }).unwrap();
        assert_eq!(read_from(&path).unwrap().tracks[0], PathBuf::from("third.wav"));
        assert_eq!(read_one(&path.with_extension("json.bak")).unwrap()
            .unwrap().tracks[0], PathBuf::from("first.flac"));
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.bak")).unwrap();
    }

    #[test]
    fn named_playlists_and_custom_eq_survive_restart() {
        let path = temp_path();
        let stored = SavedSession {
            tracks: vec![PathBuf::from("one.flac")],
            selected: Some(0),
            playlists: vec![NamedPlaylist {
                name: "Для тренувань".into(),
                tracks: vec![PathBuf::from("song.mp3"), PathBuf::from("clip.wav")],
            }],
            presets: vec![crate::presets::factory()[3].clone()],
            last_eq: crate::presets::factory()[4].clone(),
        };
        write_to(&path, &stored).unwrap();
        let recovered = read_from(&path).unwrap();
        assert_eq!(recovered.playlists, stored.playlists);
        assert_eq!(recovered.presets, stored.presets);
        assert_eq!(recovered.last_eq, stored.last_eq);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn old_v1_session_works_without_new_fields() {
        let path = temp_path();
        fs::write(&path, br#"{"version":1,"tracks":["previous.mp3"],"selected":0}"#).unwrap();
        let restored = read_from(&path).unwrap();
        assert_eq!(restored.tracks.len(), 1);
        assert!(restored.playlists.is_empty());
        assert!(restored.presets.is_empty());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn never_restore_out_of_range_selection() {
        let state = SavedSession {
            tracks: vec![PathBuf::from("song.mp3")], selected: Some(150),
            ..SavedSession::default()

        };
        assert_eq!(state.sanitized().selected, None);
    }
}
