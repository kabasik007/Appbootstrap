//! Local playlist/queue domain: no Slint dependency and no I/O.
use std::path::PathBuf;
#[derive(Debug, Default)]
pub struct PlayQueue {
    files: Vec<PathBuf>,
    selected: Option<usize>,
}
impl PlayQueue {
    pub fn clear(&mut self) { self.files.clear(); self.selected = None; }
    pub fn append(&mut self, batch: Vec<PathBuf>) {
        let capacity = 50_000usize.saturating_sub(self.files.len());
        self.files.extend(batch.into_iter().take(capacity));
    }
    pub fn append_selected(&mut self, path: PathBuf) {
        if let Some(index) = self.files.iter().position(|p| p == &path) {
            self.selected = Some(index);
        } else if self.files.len() < 50_000 {
            self.files.push(path);
            self.selected = Some(self.files.len() - 1);
        }
    }
    pub fn count(&self) -> usize { self.files.len() }
    pub fn snapshot(&self) -> Vec<PathBuf> { self.files.clone() }
    pub fn select(&mut self, index: usize) -> Option<PathBuf> {
        let result = self.files.get(index)?.clone();
        self.selected = Some(index);
        Some(result)
    }
    pub fn next(&mut self) -> Option<PathBuf> {
        if self.files.is_empty() { return None; }
        self.select(self.selected.map_or(0, |i| (i+1) % self.files.len()))
    }
    pub fn previous(&mut self) -> Option<PathBuf> {
        if self.files.is_empty() { return None; }
        self.select(self.selected.map_or(0, |i| (i+self.files.len()-1) % self.files.len()))
    }
    pub fn preview(&self, n: usize) -> Vec<String> {
        self.files.iter().take(n).map(|p| p.file_name().unwrap_or_default().to_string_lossy().into_owned()).collect()
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn cycling() {
        let mut q = PlayQueue::default();
        assert!(q.next().is_none());
        q.append(vec!["a.mp3".into(), "b.mp3".into()]);
        assert_eq!(q.next(), Some("a.mp3".into()));
        assert_eq!(q.next(), Some("b.mp3".into()));
        assert_eq!(q.next(), Some("a.mp3".into()));
        assert_eq!(q.previous(), Some("b.mp3".into()));
        assert!(q.select(100).is_none());
        q.clear(); assert_eq!(q.count(), 0);
    }
}
