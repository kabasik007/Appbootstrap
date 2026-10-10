//! M3U/M3U8 playlist import/export on background threads (no UI toolkit).
//! UTF-8, #EXTM3U/#EXTINF, relative paths and Windows absolute paths supported.
use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

const MAX_PLAYLIST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TRACKS: usize = 50_000;

pub fn parse<R: BufRead>(mut input: R, location: &Path) -> io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    let mut line = String::new();
    let mut total = 0usize;
    let base = location.parent().unwrap_or_else(|| Path::new("."));
    loop {
        line.clear();
        let size = input.read_line(&mut line)?;
        if size == 0 { break; }
        total = total.saturating_add(size);
        if total as u64 > MAX_PLAYLIST_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "M3U playlist exceeds 16 MiB"));
        }
        let entry = line.trim_end_matches(&['\r', '\n'][..]).trim_start_matches('\u{feff}');
        if entry.is_empty() || entry.starts_with('#') { continue; }
        if result.len() >= MAX_TRACKS {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "M3U playlist exceeds 50,000 entries"));
        }
        // Relative entries resolve against the *playlist's* directory.
        let path = PathBuf::from(entry);
        result.push(if path.is_absolute() { path } else { base.join(path) });
    }
    Ok(result)
}

pub fn read(path: &Path) -> io::Result<Vec<PathBuf>> {
    if fs::metadata(path)?.len() > MAX_PLAYLIST_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "M3U playlist exceeds 16 MiB"));
    }
    parse(BufReader::new(File::open(path)?), path)
}

pub fn write(path: &Path, tracks: &[PathBuf]) -> io::Result<()> {
    if tracks.len() > MAX_TRACKS {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Too many playlist entries"));
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let tmp = path.with_extension("m3u8.tmp");
    let backup = path.with_extension("m3u8.bak");
    let result = (|| -> io::Result<()> {
        let mut file = BufWriter::new(File::create(&tmp)?);
        writeln!(file, "#EXTM3U")?;
        let mut length = "#EXTM3U\n".len();
        for track in tracks {
            let line = track.to_string_lossy();
            // Newlines would inject additional playlist entries. Windows normally
            // forbids them; explicitly reject on all operating systems.
            if line.contains('\n') || line.contains('\r') {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Newline in track path"));
            }
            length = length.saturating_add(line.len() + 1);
            if length as u64 > MAX_PLAYLIST_BYTES {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Export exceeds 16 MiB"));
            }
            writeln!(file, "{line}")?;
        }
        file.flush()?;
        drop(file);
        if path.exists() {
            if backup.exists() { fs::remove_file(&backup)?; }
            fs::rename(path, &backup)?;
        }
        if let Err(err) = fs::rename(&tmp, path) {
            if backup.exists() { let _ = fs::rename(&backup, path); }
            return Err(err);
        }
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(tmp); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_bom_comments_unicode_and_relative_paths() {
        let content = "\u{feff}#EXTM3U\r\n#EXTINF:180,Artist - Song\nтрек.mp3\r\nnested/song.flac\n";
        let location = Path::new("C:/Music/list.m3u8");
        let output = parse(Cursor::new(content), location).unwrap();
        assert_eq!(output.len(), 2);
        assert!(output[0].ends_with("трек.mp3"));
        assert!(output[1].ends_with("nested/song.flac"));
    }

    #[test]
    fn ignores_empty_and_metadata_lines() {
        let output = parse(Cursor::new("#EXTM3U\n#EXTINF:30,test\n\nx.wav\n"), Path::new("list.m3u")).unwrap();
        assert_eq!(output.len(), 1);
    }

    #[test]
    fn m3u_round_trip_preserves_unicode() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("zillaplayer-list-{}-{unique}.m3u8", std::process::id()));
        let original = vec![path.with_file_name("Пісня.mp3"),
                            path.with_file_name("album.flac")];
        write(&path, &original).unwrap();
        assert_eq!(read(&path).unwrap(), original);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_oversized_input() {
        let big = format!("{}\n", "x".repeat(MAX_PLAYLIST_BYTES as usize));
        assert!(parse(Cursor::new(big), Path::new("list.m3u8")).is_err());
    }
}
