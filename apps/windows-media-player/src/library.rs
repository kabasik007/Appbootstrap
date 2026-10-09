//! Folder scanning runs in a separate CHILD PROCESS, never on UI or audio threads.
//! IPC is newline-delimited JSON with bounded batches and cancellation.
use std::{fs, io::{BufRead, BufReader, BufWriter, Write}, path::{Path, PathBuf},
          process::{Command, Stdio}, sync::{Arc, atomic::{AtomicU64, Ordering},
          mpsc::{sync_channel, Receiver, SyncSender}}, thread};

const MAX_TRACKS: usize = 50_000;
const BATCH: usize = 64;

#[derive(Debug)]
pub enum ScanEvent {
    Batch(u64, Vec<PathBuf>),
    Done(u64, usize),
    Error(u64, String),
}
pub struct LibraryScanner {
    generation: Arc<AtomicU64>,
    sender: SyncSender<ScanEvent>,
    pub updates: Receiver<ScanEvent>,
}
impl LibraryScanner {
    pub fn new() -> Self {
        let (sender, updates) = sync_channel(64);
        Self { generation: Arc::new(AtomicU64::new(0)), sender, updates }
    }
    pub fn scan(&self, folder: PathBuf) -> u64 {
        let serial = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let generation = Arc::clone(&self.generation);
        let sender = self.sender.clone();
        thread::spawn(move || read_child(folder, serial, generation, sender));
        serial
    }
    pub fn cancel(&self) { self.generation.fetch_add(1, Ordering::AcqRel); }
}
fn read_child(folder: PathBuf, serial: u64, generation: Arc<AtomicU64>, sender: SyncSender<ScanEvent>) {
    let Ok(exe) = std::env::current_exe() else {
        let _ = sender.send(ScanEvent::Error(serial, "Cannot locate scanner executable".into())); return;
    };
    let mut child = match Command::new(exe).arg("--scan-worker").arg(folder)
        .stdin(Stdio::null()).stderr(Stdio::null()).stdout(Stdio::piped()).spawn() {
        Ok(child) => child,
        Err(err) => { let _ = sender.send(ScanEvent::Error(serial, err.to_string())); return; }
    };
    let mut batch = Vec::with_capacity(BATCH);
    let mut count = 0_usize;
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines() {
            if generation.load(Ordering::Acquire) != serial { break; }
            let Ok(line) = line else { break };
            if let Ok(Some(path)) = serde_json::from_str::<Option<String>>(&line) {
                batch.push(PathBuf::from(path));
                count += 1;
                if batch.len() == BATCH {
                    if sender.send(ScanEvent::Batch(serial, std::mem::take(&mut batch))).is_err() {
                        break;
                    }
                }
            }
        }
    }
    if generation.load(Ordering::Acquire) != serial {
        let _ = child.kill();
        let _ = child.wait();
        return;
    }
    let exit = child.wait();
    if !batch.is_empty() { let _ = sender.send(ScanEvent::Batch(serial, batch)); }
    match exit {
        Ok(status) if status.success() => { let _ = sender.send(ScanEvent::Done(serial, count)); }
        result => { let _ = sender.send(ScanEvent::Error(serial, format!("Scanner failed: {result:?}"))); }
    }
}
pub fn worker_entry(folder: &Path) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut stack = vec![(folder.to_path_buf(), 0usize)];
    let mut count = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            if count >= MAX_TRACKS { break; }
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_symlink() { continue; } // do not follow reparse loops
            let path = entry.path();
            if kind.is_dir() && depth < 64 {
                stack.push((path, depth + 1));
            } else if kind.is_file() && is_audio(&path) {
                writeln!(out, "{}", serde_json::to_string(&Some(path.to_string_lossy().to_string())).map_err(std::io::Error::other)?)?;
                count += 1;
                if count % BATCH == 0 { out.flush()?; }
            }
        }
        if count >= MAX_TRACKS { break; }
    }
    out.flush()
}
pub fn is_audio(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|extension| matches!(
        extension.to_ascii_lowercase().as_str(), "mp3"|"flac"|"wav"|"ogg"|"m4a"|"aac"|"opus"|"aiff"
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn extensions() {
        assert!(is_audio(Path::new("SONG.MP3")));
        assert!(is_audio(Path::new("mix.FlAc")));
        assert!(!is_audio(Path::new("readme.txt")));
    }
    #[test]
    fn scanner_protocol_round_trip_handles_unicode_and_newlines() {
        let path = "C:\\\\Music\\Привіт 🎵\\strange\\nname.mp3";
        let json = serde_json::to_string(&Some(path.to_owned())).unwrap();
        let parsed: Option<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_deref(),Some(path));
        assert!(!json.contains('\\n')); // JSON line must not contain physical newlines
    }
}
