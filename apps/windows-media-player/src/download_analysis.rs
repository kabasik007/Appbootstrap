//! Metadata-only preview of permitted media/playlist URLs.
//! All yt-dlp work is confined to a separate thread and subprocess; no
//! downloaded media, cookies, account auth, or DRM bypass.
use crate::downloader::{executable, validate_url};
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

pub const PREVIEW_LIMIT: usize = 100;
const OUTPUT_LIMIT: usize = 2 * 1024 * 1024;
const ANALYZE_TIMEOUT: Duration = Duration::from_secs(35);

#[derive(Clone, Debug)]
pub struct PreviewItem {
    pub position: usize,  // 1-based playlist entry, not a video ID.
    pub title: String,
    pub duration: Option<u64>,
    pub selected: bool,
}

#[derive(Clone, Debug)]
pub struct Preview {
    pub source_url: String,
    pub is_playlist: bool,
    pub title: String,
    pub items: Vec<PreviewItem>,
}
fn read_limited<R: Read>(mut reader: R) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    let mut temp = [0_u8; 8192];
    let mut exceeded = false;
    loop {
        let n = reader.read(&mut temp).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        if data.len().saturating_add(n) <= OUTPUT_LIMIT {
            data.extend_from_slice(&temp[..n]);
        } else { exceeded = true; }
        // Always drain the pipe to avoid deadlocking yt-dlp on a full stdout.
    }
    if exceeded { return Err("Список занадто великий для попереднього перегляду".into()); }
    Ok(data)
}

pub fn analyze(url: String, playlist: bool) -> Result<Preview, String> {
    let url = validate_url(&url)?;
    let exe = executable("yt-dlp");
    let mut command = Command::new(exe);
    command.args([
        "--ignore-config", "--no-warnings", "--skip-download", "--flat-playlist",
        "--dump-single-json", "--playlist-end", "100",
    ]);
    command.arg(if playlist { "--yes-playlist" } else { "--no-playlist" });
    command.arg("--").arg(&url);
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().map_err(|err|
        format!("Не вдалося запустити yt-dlp. Перевірте встановлення: {err}")
    )?;
    let stdout = child.stdout.take().ok_or("Немає каналу виводу yt-dlp")?;
    let (tx, rx) = mpsc::sync_channel(1);
    let reader = thread::spawn(move || { let _ = tx.send(read_limited(stdout)); });
    let deadline = Instant::now() + ANALYZE_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err("Час аналізу минув (35 с). Перевірте URL/yt-dlp.".into());
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(format!("Помилка процесу yt-dlp: {err}"));
            }
        }
    };
    let result = rx.recv_timeout(Duration::from_secs(2))
        .map_err(|_| "Не вдалося прочитати відповідь yt-dlp")??;
    let _ = reader.join();
    if !status {
        return Err("yt-dlp не зміг отримати метадані. Перевірте посилання та підтримку джерела.".into());
    }
    parse_preview(&url, playlist, &result)
}

pub fn spawn_preview(
    url: String, playlist: bool,
) -> Receiver<Result<Preview, String>> {
    let (tx, rx) = mpsc::sync_channel(1);
    let _ = thread::Builder::new().name("zilla-download-analyze".into())
        .spawn(move || { let _ = tx.send(analyze(url, playlist)); });
    rx
}

pub fn parse_preview(url: &str, is_playlist: bool, bytes: &[u8]) -> Result<Preview, String> {
    let data: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| "Некоректні метадані від yt-dlp".to_string())?;
    let name = data.get("title").and_then(|x| x.as_str()).unwrap_or("Медіафайл");
    let title: String = name.chars().take(130).collect();
    let mut items = Vec::new();
    if is_playlist {
        if let Some(entries) = data.get("entries").and_then(|x| x.as_array()) {
            for (i, entry) in entries.iter().take(PREVIEW_LIMIT).enumerate() {
                if entry.is_null() { continue; }
                // The list may omit unavailable items; preserve their actual
                // positions for yt-dlp's --playlist-items instead of reindexing.
                let index = entry.get("playlist_index").and_then(|x| x.as_u64())
                    .and_then(|x| usize::try_from(x).ok())
                    .filter(|x| *x > 0 && *x <= 200)
                    .unwrap_or(i + 1);
                let label = entry.get("title").and_then(|x| x.as_str()).unwrap_or("Без назви");
                items.push(PreviewItem {
                    position: index,
                    title: label.chars().take(110).collect(),
                    duration: entry.get("duration").and_then(|x| x.as_u64()),
                    selected: true,
                });
            }
        }
    }
    if items.is_empty() && !is_playlist {
        items.push(PreviewItem {
            position: 1,
            title: title.clone(),
            duration: data.get("duration").and_then(|x| x.as_u64()),
            selected: true,
        });
    }
    if items.is_empty() {
        return Err("У плейлисті не знайдено доступних позицій".into());
    }
    Ok(Preview { source_url: url.into(), is_playlist, title, items })
}

pub fn row_title(item: &PreviewItem) -> String {
    let time = item.duration.map(|s| format!(" · {}:{:02}", s / 60, s % 60))
        .unwrap_or_default();
    format!("{} {:>3}. {}{}",
        if item.selected { "☑" } else { "☐" },
        item.position, item.title, time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_playlist_entries_and_preserves_position() {
        let json = br#"{"title":"Practice list","entries":[
            {"title":"First","playlist_index":1,"duration":120},
            null,
            {"title":"Third","playlist_index":3,"duration":80}
        ]}"#;
        let preview = parse_preview("https://example.org/pl", true, json).unwrap();
        assert_eq!(preview.title, "Practice list");
        assert_eq!(preview.items.len(), 2);
        assert_eq!(preview.items[1].position, 3);
        assert!(preview.items.iter().all(|x| x.selected));
        assert!(row_title(&preview.items[1]).contains("1:20"));
    }

    #[test]
    fn single_video_preview_is_not_a_playlist() {
        let p = parse_preview("https://example.org/one",false,
            br#"{"title":"One title","duration":125}"#).unwrap();
        assert_eq!(p.items.len(), 1);
        assert_eq!(p.items[0].title, "One title");
    }

    #[test]
    fn empty_playlist_and_corrupt_json_are_rejected() {
        assert!(parse_preview("https://example.org/pl",true,br#"{"entries":[]}"#).is_err());
        assert!(parse_preview("https://example.org/pl",true,b"no-json").is_err());
    }
}
