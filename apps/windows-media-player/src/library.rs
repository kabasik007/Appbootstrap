//! Folder scanning runs in a separate CHILD PROCESS, never on UI or audio threads.
//! IPC is newline-delimited JSON with bounded batches and cancellation.
use std::{fs, io::{BufRead, BufReader, BufWriter, Write}, path::{Path, PathBuf},
          process::{Command, Stdio}, sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering},
          mpsc::{sync_channel, Receiver, SyncSender, TrySendError}}, thread, time::Duration};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

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
// Do not get permanently stuck sending into a full UI event queue.
// On cancellation, return so the supervisor can kill/wait for its child.
fn deliver(sender: &SyncSender<ScanEvent>, generation: &AtomicU64, serial: u64,
           mut event: ScanEvent) -> bool {
    loop {
        if generation.load(Ordering::Acquire) != serial { return false; }
        match sender.try_send(event) {
            Ok(()) => return true,
            Err(TrySendError::Disconnected(_)) => return false,
            Err(TrySendError::Full(returned)) => {
                event = returned;
                thread::sleep(Duration::from_millis(15));
            }
        }
    }
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
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            let _ = deliver(&sender, &generation, serial, ScanEvent::Error(serial, err.to_string()));
            return;
        }
    };
    let mut command = Command::new(exe);
    command.arg("--scan-worker").arg(folder)
        .stdin(Stdio::null()).stderr(Stdio::null()).stdout(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(0x0000_4000); // BELOW_NORMAL_PRIORITY_CLASS

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            let _ = deliver(&sender, &generation, serial, ScanEvent::Error(serial, err.to_string()));
            return;
        }
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        let _ = deliver(&sender, &generation, serial, ScanEvent::Error(serial, "Scanner has no stdout".into()));
        return;
    };

    // One thread reads stdout. The supervisor alone owns Child and polls try_wait.
    // No mutex can be held inside wait(), so cancellation can always call kill().
    let reader_gen = Arc::clone(&generation);
    let reader_tx = sender.clone();
    let aborted = Arc::new(AtomicBool::new(false));
    let reader_aborted = Arc::clone(&aborted);
    let reader = thread::Builder::new().name("zillaplayer-scanner-ipc".into()).spawn(move || {
        let mut batch = Vec::with_capacity(BATCH);
        let mut count = 0_usize;
        let result = (|| -> Result<(), String> {
            for line in BufReader::new(stdout).lines() {
                if reader_gen.load(Ordering::Acquire) != serial { return Err("Scan canceled".into()); }
                let line = line.map_err(|e| e.to_string())?;
                let file: Option<String> = serde_json::from_str(&line).map_err(|e| e.to_string())?;
                if let Some(file) = file {
                    batch.push(PathBuf::from(file));
                    count += 1;
                    if batch.len() == BATCH && !deliver(&reader_tx, &reader_gen, serial,
                        ScanEvent::Batch(serial, std::mem::take(&mut batch))) {
                        return Err("Scanner IPC disconnected".into());
                    }
                }
            }
            if !batch.is_empty() && !deliver(&reader_tx, &reader_gen, serial,
                ScanEvent::Batch(serial, batch)) {
                return Err("Scanner IPC disconnected".into());
            }
            Ok(())
        })();
        if result.is_err() { reader_aborted.store(true, Ordering::Release); }
        result.map(|_| count)
    });
    let reader = match reader {
        Ok(handle) => handle,
        Err(err) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = deliver(&sender, &generation, serial, ScanEvent::Error(serial, err.to_string()));
            return;
        }
    };

    let exit = loop {
        if generation.load(Ordering::Acquire) != serial || aborted.load(Ordering::Acquire) {
            let _ = child.kill();
            break child.wait();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => thread::sleep(Duration::from_millis(35)),
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(err);
            }
        }
    };
    // On cancellation TerminateProcess closes stdout; the reader finishes before
    // we send the terminal event. This preserves the order of Batch then Done.
    let scan_result = reader.join();
    if generation.load(Ordering::Acquire) != serial { return; }
    match (exit, scan_result) {
        (Ok(status), Ok(Ok(count))) if status.success() => {
            let _ = deliver(&sender, &generation, serial, ScanEvent::Done(serial, count));
        }
        (status, reader) => {
            let reason = format!("Scanner failed: process={status:?}, reader={reader:?}");
            let _ = deliver(&sender, &generation, serial, ScanEvent::Error(serial, reason));
        }
    }
}

/// Scanner entry point for the separate subprocess. No GUI or audio initialized.
pub fn worker_entry(folder: &Path) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    scan_to_writer(folder, BufWriter::new(stdout.lock())).map(|_| ())
}

/// Reusable scanning primitive: depth/count bounds and no symlink recursion.
/// Generic writer makes scanner logic testable without running a GUI process.
fn scan_to_writer<W: Write>(folder: &Path, mut output: W) -> std::io::Result<usize> {
    let mut stack = vec![(folder.to_path_buf(), 0usize)];
    let mut found = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries {
            if found >= MAX_TRACKS { break; }
            let Ok(entry) = entry else { continue };
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_symlink() { continue; }
            let path = entry.path();
            if kind.is_dir() {
                if depth < 64 { stack.push((path, depth + 1)); }
            } else if kind.is_file() && is_audio(&path) {
                let line = serde_json::to_string(&Some(path.to_string_lossy().to_string()))
                    .map_err(std::io::Error::other)?;
                writeln!(output, "{line}")?;
                found += 1;
                if found % BATCH == 0 { output.flush()?; }
            }
        }
        if found >= MAX_TRACKS { break; }
    }
    output.flush()?;
    Ok(found)
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
        let path = "C:/Music/Привіт 🎵/strange\nname.mp3";
        let json = serde_json::to_string(&Some(path.to_owned())).unwrap();
        let parsed: Option<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_deref(), Some(path));
        assert!(!json.contains('\n')); // No literal newline in JSONL records
        assert!(json.contains("\\n")); // Escaped newline survives serialization
    }
}
