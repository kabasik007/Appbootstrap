//! Optional local media downloader. yt-dlp and FFmpeg run as external child
//! processes; no network/decode/transcode work enters Slint or the audio thread.
//!
//! Sources must be permitted to download; no cookies/DRM/auth bypass.
//! Job queue, progress and failures persist locally. A single actor owns the
//! child process; cancellation and UI snapshots never wait on its stdout pipe.
use std::{
    fs,
    io::{BufRead, BufReader},
    net::IpAddr,
    path::{Path, PathBuf},
    process::{Child, Command as ProcessCommand, Stdio},
    sync::{mpsc::{self, Receiver, Sender, SyncSender, TryRecvError, TrySendError}, Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const MAX_JOBS: usize = 64;
const MAX_PLAYLIST_ITEMS: usize = 200;
const MAX_URL_LEN: usize = 2048;
const MAX_LOG: usize = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Mp3,
    OriginalAudio,
    Video1440,
    Video1080,
    BestVideo,
}
impl Format {
    pub fn by_index(index: i32) -> Self {
        match index {
            1 => Self::OriginalAudio, 2 => Self::Video1440,
            3 => Self::Video1080, 4 => Self::BestVideo, _ => Self::Mp3,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3", Self::OriginalAudio => "original-audio",
            Self::Video1440 => "video-1440p", Self::Video1080 => "video-1080p",
            Self::BestVideo => "best-video",
        }
    }
    fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "mp3" => Self::Mp3,
            "original-audio" => Self::OriginalAudio,
            "video-1440p" => Self::Video1440,
            "video-1080p" => Self::Video1080,
            "best-video" => Self::BestVideo,
            _ => return None,
        })
    }
    fn needs_ffmpeg(self) -> bool { true }
    fn is_audio(self) -> bool { matches!(self, Self::Mp3 | Self::OriginalAudio) }
    pub fn label(self) -> &'static str {
        match self {
            Self::Mp3 => "MP3 320k",
            Self::OriginalAudio => "Аудіо (оригінал)",
            Self::Video1440 => "QHD 1440p / MKV",
            Self::Video1080 => "Full HD 1080p / MKV",
            Self::BestVideo => "Відео / найкраще / MKV",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Queued, Running, Finished, Failed, Canceled, Interrupted }
impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "Черга", Self::Running => "Завантажується",
            Self::Finished => "Готово", Self::Failed => "Помилка",
            Self::Canceled => "Скасовано", Self::Interrupted => "Перервано",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Queued => "queued", Self::Running => "running",
            Self::Finished => "finished", Self::Failed => "failed",
            Self::Canceled => "canceled", Self::Interrupted => "interrupted",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "queued" => Self::Queued, "running" => Self::Running,
            "finished" => Self::Finished, "failed" => Self::Failed,
            "canceled" => Self::Canceled, "interrupted" => Self::Interrupted,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Job {
    pub id: u64,
    pub url: String,
    pub format: Format,
    pub playlist: bool,
    pub folder: PathBuf,
    pub phase: Phase,
    pub progress: f32,
    pub detail: String,
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub jobs: Vec<Job>,
    pub tools: String,
    pub message: String,
    pub imported_files: Vec<PathBuf>,
}

enum Action {
    Queue { url: String, format: Format, playlist: bool, folder: PathBuf },
    Cancel(usize),
    Retry(usize),
    ClearFinished,
    Probe,
    Shutdown,
}
enum Output {
    Progress(u64, f32),
    File(u64, PathBuf),
    Title(u64, String),
    Error(u64, String),
}
struct Active {
    id: u64,
    child: Child,
    started: Instant,
    readers: Vec<JoinHandle<()>>,
    cancelled: bool,
}

pub struct Downloader {
    command: Sender<Action>,
    state: Arc<Mutex<Snapshot>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Downloader {
    pub fn start() -> Self {
        let (command, receiver) = mpsc::channel();
        let state = Arc::new(Mutex::new(Snapshot {
            message: "yt-dlp / FFmpeg — перевірте інструменти".into(),
            ..Snapshot::default()
        }));
        let worker_state = Arc::clone(&state);
        let worker = thread::Builder::new().name("zilla-download-supervisor".into())
            .spawn(move || supervise(receiver, worker_state)).ok();
        Self { command, state, worker: Mutex::new(worker) }
    }
    pub fn queue(&self, url: String, format: Format, playlist: bool, folder: PathBuf) {
        let _ = self.command.send(Action::Queue { url, format, playlist, folder });
    }
    pub fn cancel(&self, index: usize) { let _ = self.command.send(Action::Cancel(index)); }
    pub fn retry(&self, index: usize) { let _ = self.command.send(Action::Retry(index)); }
    pub fn clear_finished(&self) { let _ = self.command.send(Action::ClearFinished); }
    pub fn probe(&self) { let _ = self.command.send(Action::Probe); }
    pub fn snapshot(&self) -> Snapshot {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    /// Drain completed local audio files exactly once for the Library UI.
    pub fn take_imported(&self) -> Vec<PathBuf> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut state.imported_files)
    }
    pub fn shutdown(&self) {
        let _ = self.command.send(Action::Shutdown);
        if let Some(worker) = self.worker.lock().unwrap_or_else(|e|e.into_inner()).take() {
            let _ = worker.join();
        }
    }
}

fn validate_url(raw: &str) -> Result<String, String> {
    if raw.len() > MAX_URL_LEN || raw.is_empty() ||
        raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("Неправильна або занадто довга URL-адреса".into());
    }
    let parsed = url::Url::parse(raw).map_err(|_| "Неправильна URL-адреса")?;
    if !["https", "http"].contains(&parsed.scheme()) {
        return Err("Дозволено лише HTTPS / HTTP".into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("URL з логіном або паролем не підтримується".into());
    }
    let host = parsed.host_str().ok_or("URL без хоста")?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err("Локальні URL заборонені".into());
    }
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
        let forbidden = match ip {
            IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() ||
                ip.is_link_local() || ip.is_broadcast() || ip.is_unspecified(),
            IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local() ||
                ip.is_unicast_link_local() || ip.is_unspecified(),
        };
        if forbidden { return Err("Локальні IP-адреси заборонені".into()); }
    }
    Ok(parsed.into())
}
fn default_path() -> PathBuf {
    let app = std::env::var_os("APPDATA").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    app.join("ZillaPlayer").join("downloads-v1.json")
}
pub fn suggested_folder() -> PathBuf {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir).join("Downloads").join("ZillaPlayer")
}
fn save_jobs(path: &Path, jobs: &[Job]) -> Result<(), String> {
    let value = serde_json::json!({
        "version": 1,
        "jobs": jobs.iter().map(|j| serde_json::json!({
            "id":j.id,"url":j.url,"format":j.format.as_str(),
            "playlist":j.playlist,"folder":j.folder.to_string_lossy(),
            "phase":j.phase.key(),"progress":j.progress,
            "detail":j.detail,
        })).collect::<Vec<_>>()
    });
    let json = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    if json.len() > 200_000 { return Err("Занадто велика історія".into()); }
    let parent = path.parent().ok_or("Шлях до історії не визначено")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    // On Windows rename cannot replace destination; use backup/rollback.
    let backup = path.with_extension("json.bak");
    if path.exists() {
        if backup.exists() { fs::remove_file(&backup).map_err(|e| e.to_string())?; }
        fs::rename(path, &backup).map_err(|e| e.to_string())?;
    }
    if let Err(err) = fs::rename(&tmp, path) {
        if backup.exists() { let _ = fs::rename(&backup, path); }
        return Err(err.to_string());
    }
    Ok(())
}
fn load_jobs(path: &Path) -> Vec<Job> {
    let value = [path.to_path_buf(),path.with_extension("json.bak")]
        .iter().find_map(|candidate| {
            let bytes=fs::read(candidate).ok()?;
            if bytes.len()>200_000 { return None; }
            let value: serde_json::Value=serde_json::from_slice(&bytes).ok()?;
            if value.get("version").and_then(|v|v.as_u64())!=Some(1) { return None; }
            Some(value)
        });
    let Some(value)=value else { return Vec::new(); };
    value.get("jobs").and_then(|v| v.as_array())
        .into_iter().flatten().take(MAX_JOBS)
        .filter_map(|x| {
            let url = validate_url(x.get("url")?.as_str()?).ok()?;
            let format = Format::from_str(x.get("format")?.as_str()?)?;
            let id = x.get("id")?.as_u64()?;
            let folder = PathBuf::from(x.get("folder")?.as_str()?);
            let phase = Phase::parse(x.get("phase")?.as_str()?)?;
            Some(Job {
                id,url,format,folder,
                playlist:x.get("playlist").and_then(|v|v.as_bool()).unwrap_or(false),
                phase:if matches!(phase,Phase::Queued|Phase::Running) { Phase::Interrupted } else { phase },
                progress:x.get("progress").and_then(|v|v.as_f64()).unwrap_or(0.0) as f32,
                detail:x.get("detail").and_then(|v|v.as_str()).unwrap_or("").chars().take(MAX_LOG).collect(),
            })
        }).collect()
}
fn candidate_locations(name: &str, executable_dir: Option<&Path>) -> Vec<PathBuf> {
    #[cfg(windows)]
    let bin = format!("{name}.exe");
    #[cfg(not(windows))]
    let bin = name.to_string();
    let mut choices = Vec::new();
    if let Some(dir) = executable_dir {
        choices.extend([
            dir.join(&bin),
            dir.join("tools").join(&bin),
            dir.join("tools").join(name).join(&bin),
            dir.join("tools").join("ffmpeg").join("bin").join(&bin),
        ]);
    }
    #[cfg(windows)]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let local = PathBuf::from(local);
            choices.push(local.join("Microsoft").join("WinGet").join("Links").join(&bin));
            choices.push(local.join("Programs").join(name).join(&bin));
            if name == "deno" {
                choices.push(local.join("deno").join(&bin));
            }
        }
        if let Some(user) = std::env::var_os("USERPROFILE") {
            let user = PathBuf::from(user);
            if name == "deno" {
                choices.push(user.join(".deno").join("bin").join(&bin));
            }
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        choices.extend(std::env::split_paths(&paths).map(|p| p.join(&bin)));
    }
    choices
}
fn executable(name: &str) -> PathBuf {
    let exe_dir = std::env::current_exe().ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    candidate_locations(name, exe_dir.as_deref()).into_iter()
        .find(|p| p.is_file())
        .unwrap_or_else(|| {
            #[cfg(windows)]
            { PathBuf::from(format!("{name}.exe")) }
            #[cfg(not(windows))]
            { PathBuf::from(name) }
        })
}
fn probe_tool(name: &str) -> bool {
    ProcessCommand::new(executable(name))
        .arg(if name == "ffmpeg" { "-version" } else { "--version" })
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .status().is_ok_and(|status| status.success())
}
fn probes() -> String {
    let yt = probe_tool("yt-dlp");
    let ff = probe_tool("ffmpeg");
    let deno = probe_tool("deno");
    // Deno is an optional runtime for some modern extractors, not a general
    // precondition for queueing downloads.
    format!("yt-dlp: {}  •  FFmpeg: {}  •  Deno: {}",
        if yt{"OK"}else{"НЕ ЗНАЙДЕНО"},
        if ff{"OK"}else{"НЕ ЗНАЙДЕНО"},
        if deno{"OK (необов’язково)"}else{"НЕ ЗНАЙДЕНО (необов’язково)"})
}

/// Fixed command vector. No shell, cookies, arbitrary user args or local files.
/// 1440p is QHD (2560x1440), sometimes marketed as "2K".
fn arguments(job: &Job) -> Vec<String> {
    let mut args = [
        "--ignore-config", "--no-overwrites", "--continue",
        "--newline", "--no-color", "--no-mtime",
        "--output", "%(title).180B [%(id)s].%(ext)s",
        "--progress-template", "download:ZILLA_PROGRESS:%(progress._percent_str)s",
        "--print", "after_move:ZILLA_FILE:%(filepath)s",
        "--print", "before_dl:ZILLA_TITLE:%(title)s",
    ].into_iter().map(str::to_string).collect::<Vec<_>>();
    args.push("--paths".into());
    args.push(job.folder.to_string_lossy().to_string());
    // yt-dlp cannot locate a bundled FFmpeg merely because our own probe can.
    // Pass its verified exact executable location when it is outside PATH.
    let ffmpeg = executable("ffmpeg");
    if ffmpeg.is_file() {
        args.push("--ffmpeg-location".into());
        args.push(ffmpeg.to_string_lossy().to_string());
    }
    if job.playlist {
        args.extend(["--yes-playlist", "--playlist-end", "200"].map(str::to_string));
    } else {
        args.push("--no-playlist".into());
    }
    match job.format {
        Format::Mp3 => args.extend(["-f","ba/b","-x","--audio-format","mp3",
                                    "--audio-quality","320K"].map(str::to_string)),
        Format::OriginalAudio => args.extend(["-f","ba/b","-x",
                                               "--audio-format","best"].map(str::to_string)),
        Format::Video1440 => args.extend(["-f","bv*[height<=1440]+ba/b[height<=1440]",
                                          "--merge-output-format","mkv"].map(str::to_string)),
        Format::Video1080 => args.extend(["-f","bv*[height<=1080]+ba/b[height<=1080]",
                                          "--merge-output-format","mkv"].map(str::to_string)),
        Format::BestVideo => args.extend(["-f","bv*+ba/b",
                                          "--merge-output-format","mkv"].map(str::to_string)),
    }
    args.push("--".into());
    args.push(job.url.clone());
    args
}

fn spawn_reader<R: std::io::Read + Send + 'static>(
    input: R, id: u64, tx: SyncSender<Output>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(input).lines() {
            let Ok(line) = line else { break; };
            if let Some(raw) = line.strip_prefix("ZILLA_PROGRESS:") {
                if let Some(percent) = parse_progress(raw) {
                    let _ = tx.try_send(Output::Progress(id, percent));
                }
            } else if let Some(raw) = line.strip_prefix("ZILLA_TITLE:") {
                let _ = tx.try_send(Output::Title(id, raw.chars().take(100).collect()));
            } else if let Some(raw) = line.strip_prefix("ZILLA_FILE:") {
                // File notifications are infrequent and must not be lost.
                let _ = tx.try_send(Output::File(id, PathBuf::from(raw)));
            } else if line.to_ascii_lowercase().contains("error:") {
                let detail = line.chars().take(MAX_LOG).collect::<String>();
                let _ = tx.try_send(Output::Error(id, detail));
            }
        }
    })
}
fn parse_progress(line: &str) -> Option<f32> {
    let number = line.trim().trim_end_matches('%').trim().parse::<f32>().ok()?;
    if number.is_finite() { Some(number.clamp(0., 100.)) } else { None }
}

fn stop_child(active: &mut Active) {
    // yt-dlp may spawn FFmpeg; on Windows taskkill /T terminates its tree.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = ProcessCommand::new("taskkill")
            .args(["/T","/F","/PID",&active.child.id().to_string()])
            .creation_flags(0x08000000).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    let _ = active.child.kill();
    let _ = active.child.wait();
    for reader in active.readers.drain(..) { let _ = reader.join(); }
}
fn launch(job: &Job, sender: SyncSender<Output>) -> Result<Active, String> {
    if !probe_tool("yt-dlp") {
        return Err("yt-dlp.exe не знайдено (покладіть поруч із ZillaPlayer.exe або встановіть у PATH)".into());
    }
    if job.format.needs_ffmpeg() && !probe_tool("ffmpeg") {
        return Err("FFmpeg не знайдено (потрібен для MP3 та склеювання відео+аудіо)".into());
    }
    fs::create_dir_all(&job.folder).map_err(|e|format!("Папка недоступна: {e}"))?;
    let mut cmd = ProcessCommand::new(executable("yt-dlp"));
    cmd.args(arguments(job))
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.spawn().map_err(|e|format!("Не вдалося запустити yt-dlp: {e}"))?;
    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        readers.push(spawn_reader(stdout, job.id, sender.clone()));
    }
    if let Some(stderr) = child.stderr.take() {
        readers.push(spawn_reader(stderr, job.id, sender));
    }
    Ok(Active { id:job.id,child,readers,started:Instant::now(),cancelled:false })
}
fn publish(state: &Arc<Mutex<Snapshot>>, jobs: &[Job], tools: &str, message: &str,
    delivered: Vec<PathBuf>,
) {
    let mut snapshot = state.lock().unwrap_or_else(|e|e.into_inner());
    snapshot.jobs = jobs.to_vec();
    snapshot.tools = tools.to_string();
    snapshot.message = message.to_string();
    snapshot.imported_files.extend(delivered);
    if snapshot.imported_files.len() > 1000 {
        snapshot.imported_files.drain(..500);
    }
}
fn supervise(actions: Receiver<Action>, state: Arc<Mutex<Snapshot>>) {
    let path = default_path();
    let mut jobs = load_jobs(&path);
    let mut current: Option<Active> = None;
    let mut next_id = jobs.iter().map(|j|j.id).max().unwrap_or(0).saturating_add(1);
    let mut tool_state = probes();
    let mut info = String::from("Дозволені файли: MP3, оригінальне аудіо, відео та плейлисти");
    let (out, updates) = mpsc::sync_channel::<Output>(256);
    let mut dirty = false;
    let mut last_save = Instant::now();
    let mut files = Vec::new();

    loop {
        match actions.recv_timeout(Duration::from_millis(90)) {
            Ok(Action::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(mut task)=current.take() {
                    stop_child(&mut task);
                    if let Some(job) = jobs.iter_mut().find(|x|x.id==task.id) {
                        job.phase=Phase::Interrupted;job.detail="Перервано після закриття програми".into();
                    }
                }
                break;
            }
            Ok(Action::Probe) => { tool_state=probes(); }
            Ok(Action::Queue{url,format,playlist,folder}) => {
                match validate_url(&url) {
                    Err(reason) => info=reason,
                    Ok(url) if jobs.len() >= MAX_JOBS => info="Ліміт 64 завдань (спочатку очистіть історію)".into(),
                    Ok(url) => {
                        // Destination comes from native folder picker and must exist or be creatable.
                        if folder.as_os_str().is_empty() {
                            info="Оберіть папку завантаження".into();
                        } else {
                            jobs.push(Job {
                                id:next_id,url,format,playlist,folder,
                                phase:Phase::Queued,progress:0.0,detail:"Очікує".into(),
                            });
                            next_id=next_id.saturating_add(1);
                            info="Додано до черги".into(); dirty=true;
                        }
                    }
                }
            }
            Ok(Action::Cancel(index)) => {
                if let Some(job)=jobs.get_mut(index) {
                    if job.phase==Phase::Running {
                        if let Some(mut process)=current.take() {
                            if process.id==job.id {
                                stop_child(&mut process);
                                job.phase=Phase::Canceled;job.detail="Скасовано користувачем".into();
                                dirty=true;
                            } else { current=Some(process); }
                        }
                    } else if job.phase==Phase::Queued {
                        job.phase=Phase::Canceled;job.detail="Вилучено з черги".into();dirty=true;
                    }
                }
            }
            Ok(Action::Retry(index)) => {
                if let Some(job)=jobs.get_mut(index) {
                    if matches!(job.phase,Phase::Failed|Phase::Canceled|Phase::Interrupted) {
                        job.phase=Phase::Queued;job.progress=0.0;
                        job.detail="Повторна спроба".into();dirty=true;
                    }
                }
            }
            Ok(Action::ClearFinished) => {
                let before = jobs.len();
                jobs.retain(|j|matches!(j.phase,Phase::Queued|Phase::Running));
                if jobs.len()!=before { dirty=true; }
                info=format!("Очищено {} завершених завдань", before-jobs.len());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        // Drain bounded stdout/stderr events. No user input blocks on output pipes.
        for _ in 0..256 {
            match updates.try_recv() {
                Ok(Output::Progress(id, progress)) => {
                    if current.as_ref().is_some_and(|task|task.id==id) {
                        if let Some(job)=jobs.iter_mut().find(|x|x.id==id) {
                            job.progress=progress;dirty=true;
                        }
                    }
                }
                Ok(Output::Title(id, title)) => {
                    if current.as_ref().is_some_and(|task|task.id==id) {
                        if let Some(job)=jobs.iter_mut().find(|x|x.id==id) {
                            job.detail=title;dirty=true;
                        }
                    }
                }
                Ok(Output::File(id, path)) => {
                    if current.as_ref().is_some_and(|task|task.id==id) {
                        if let Some(job)=jobs.iter_mut().find(|x|x.id==id) {
                            // yt-dlp --print after_move returns output after FFmpeg.
                            if job.format.is_audio() &&
                                path.is_file() && path.starts_with(&job.folder) {
                                files.push(path);
                            }
                        }
                    }
                }
                Ok(Output::Error(id, detail)) => {
                    if current.as_ref().is_some_and(|task|task.id==id) {
                        if let Some(job)=jobs.iter_mut().find(|x|x.id==id) {
                            job.detail=detail;dirty=true;
                        }
                    }
                }
                Err(TryRecvError::Empty|TryRecvError::Disconnected) => break,
            }
        }
        if let Some(task)=current.as_mut() {
            if let Ok(Some(result))=task.child.try_wait() {
                let id=task.id;
                let success=result.success();
                let elapsed=task.started.elapsed().as_secs();
                // Reap after output closed; wait for trailing after_move filenames.
                let mut process=current.take().unwrap();
                let _=process.child.wait();
                for reader in process.readers.drain(..) { let _=reader.join(); }
                for event in updates.try_iter() {
                    if let Output::File(event_id, path)=event {
                        if event_id!=id { continue; }
                        if let Some(job)=jobs.iter().find(|x|x.id==id) {
                            if job.format.is_audio() && path.is_file() && path.starts_with(&job.folder) {
                                files.push(path);
                            }
                        }
                    }
                }
                if let Some(job)=jobs.iter_mut().find(|x|x.id==id) {
                    job.phase=if success{Phase::Finished}else{Phase::Failed};
                    job.progress=if success{100.0}else{job.progress};
                    if success {
                        job.detail=format!("Готово за {} сек · {}",elapsed,job.format.label());
                        info=format!("Завершено: {}",job.format.label());
                    } else {
                        if !job.detail.to_lowercase().contains("error:") {
                            job.detail=format!("yt-dlp завершився з кодом {result}");
                        }
                        info="Помилка завантаження — перегляньте статус".into();
                    }
                    dirty=true;
                }
            }
        }
        if current.is_none() {
            if let Some(job)=jobs.iter_mut().find(|j|j.phase==Phase::Queued) {
                match launch(job,out.clone()) {
                    Ok(process)=> {
                        info=format!("Завантаження: {}",job.format.label());
                        job.phase=Phase::Running;
                        job.detail="Завантаження, FFmpeg обробить аудіодоріжку за потреби".into();
                        current=Some(process);
                    }
                    Err(reason)=> {
                        info=reason.clone();job.phase=Phase::Failed;job.detail=reason;
                    }
                }
                dirty=true;
            }
        }
        // Do not lose completion events: UI drains them from cloned snapshot.
        publish(&state,&jobs,&tool_state,&info,std::mem::take(&mut files));
        if dirty && last_save.elapsed()>Duration::from_millis(500) {
            let _=save_jobs(&path,&jobs);
            last_save=Instant::now();dirty=false;
        }
    }
    let _=save_jobs(&path,&jobs);
    publish(&state,&jobs,&tool_state,&info,files);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fake_job(format:Format,playlist:bool)->Job {
        Job{id:1,url:"https://example.org/authorized-media".into(),
            format,playlist,folder:PathBuf::from("C:/Downloads"),
            phase:Phase::Queued,progress:0.,detail:String::new()}
    }
    #[test]
    fn bundled_tool_lookup_includes_exe_and_tools_folder() {
        let root = PathBuf::from("C:/Program Files/ZillaPlayer");
        let paths = candidate_locations("yt-dlp", Some(&root));
        #[cfg(windows)]
        {
            assert_eq!(paths[0], root.join("yt-dlp.exe"));
            assert_eq!(paths[1], root.join("tools/yt-dlp.exe"));
        }
        #[cfg(not(windows))]
        {
            assert_eq!(paths[0], root.join("yt-dlp"));
            assert_eq!(paths[1], root.join("tools/yt-dlp"));
        }
    }

    #[test]
    fn validate_input_and_reject_option_injection() {
        assert!(validate_url("https://example.org/watch?v=1").is_ok());
        for url in ["--output=evil","file:///C:/Windows/test","http://localhost/","http://127.0.0.1/",
            "http://192.168.1.1","https://user:pass@example.org/","https://example.org/\n--exec"] {
            assert!(validate_url(url).is_err(), "Unsafe: {url}");
        }
    }
    #[test]
    fn qhd_merges_video_and_audio_without_shell() {
        let args=arguments(&fake_job(Format::Video1440,false));
        assert!(args.windows(2).any(|w|w[0]=="-f"&&w[1].contains("height<=1440")));
        assert!(args.windows(2).any(|w|w==["--merge-output-format","mkv"]));
        assert!(args.windows(2).any(|w|w==["--print","after_move:ZILLA_FILE:%(filepath)s"]));
        assert!(args.contains(&"--no-playlist".to_string()));
        assert_eq!(args[args.len()-2],"--");
    }
    #[test]
    fn original_audio_extracts_audio_even_from_muxed_media() {
        let args=arguments(&fake_job(Format::OriginalAudio,false));
        assert!(args.contains(&"-x".to_string()));
        assert!(args.windows(2).any(|w|w==["--audio-format","best"]));
    }
    #[test]
    fn mp3_playlist_has_transcode_and_item_cap() {
        let args=arguments(&fake_job(Format::Mp3,true));
        assert!(args.windows(2).any(|w|w==["--audio-format","mp3"]));
        assert!(args.windows(2).any(|w|w==["--audio-quality","320K"]));
        assert!(args.windows(2).any(|w|w==["--playlist-end","200"]));
    }
    #[test]
    fn progress_parser_handles_realistic_percentages() {
        assert_eq!(parse_progress(" 75.4% "),Some(75.4));
        assert_eq!(parse_progress("98.3%"),Some(98.3));
        assert_eq!(parse_progress("nan%"),None);
    }
    #[test]
    fn per_job_output_messages_keep_their_origin_id() {
        use std::io::Cursor;
        let lines = Cursor::new(b"ZILLA_PROGRESS: 44.0%\nZILLA_TITLE:Licensed Song\n".to_vec());
        let (tx,rx)=mpsc::sync_channel(8);
        let worker=spawn_reader(lines,900,tx);
        worker.join().unwrap();
        assert!(matches!(rx.try_recv(),Ok(Output::Progress(900,pct)) if pct==44.0));
        assert!(matches!(rx.try_recv(),Ok(Output::Title(900,title)) if title=="Licensed Song"));
    }

    #[test]
    fn unfinished_jobs_restore_as_interrupted_and_backup_survives() {
        use std::time::{SystemTime,UNIX_EPOCH};
        let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let folder=std::env::temp_dir().join(format!("zillaplayer-dl-{}-{stamp}",std::process::id()));
        let file=folder.join("downloads-v1.json");
        let mut job=fake_job(Format::Mp3,true);
        job.phase=Phase::Running;job.progress=23.5;
        save_jobs(&file,&[job.clone()]).unwrap();
        save_jobs(&file,&[job]).unwrap(); // Create backup.
        fs::write(&file,b"invalid-json").unwrap();
        let restored=load_jobs(&file);
        assert_eq!(restored.len(),1);
        assert_eq!(restored[0].phase,Phase::Interrupted);
        assert_eq!(restored[0].format,Format::Mp3);
        assert_eq!(restored[0].progress,23.5);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn profile_and_job_phases_roundtrip() {
        for f in [Format::Mp3,Format::OriginalAudio,Format::Video1440,Format::Video1080,Format::BestVideo] {
            assert_eq!(Format::from_str(f.as_str()),Some(f));
        }
        for p in [Phase::Queued,Phase::Running,Phase::Finished,Phase::Failed,Phase::Canceled,Phase::Interrupted] {
            assert_eq!(Phase::parse(p.key()),Some(p));
        }
    }
}
