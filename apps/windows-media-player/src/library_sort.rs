//! Lightweight library presentation ordering. Never mutates playback queue.
use std::{cmp::Ordering, collections::HashMap, path::{Path, PathBuf}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackRating {
    pub path: PathBuf,
    pub stars: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy { Queue, NameAsc, NameDesc, HighestRated, Folder }
impl SortBy {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Self::NameAsc,
            2 => Self::NameDesc,
            3 => Self::HighestRated,
            4 => Self::Folder,
            _ => Self::Queue,
        }
    }
}

pub fn rating(stars: &HashMap<PathBuf, u8>, path: &Path) -> u8 {
    stars.get(path).copied().unwrap_or(0).min(5)
}
pub fn next_rating(stars: &mut HashMap<PathBuf, u8>, path: PathBuf) -> u8 {
    let value = (rating(stars, &path) + 1) % 6;
    if value == 0 { stars.remove(&path); }
    else { stars.insert(path, value); }
    value
}

pub fn filtered_order(
    paths: &[PathBuf], stars: &HashMap<PathBuf, u8>, query: &str, sort: SortBy,
) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    let mut items: Vec<usize> = paths.iter().enumerate()
        .filter_map(|(i,p)| {
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            if query.is_empty() || name.contains(&query) { Some(i) } else { None }
        }).collect();
    if sort == SortBy::Queue { return items; }
    let lower_name = |i: usize| -> String {
        paths[i].file_name().unwrap_or_default().to_string_lossy().to_lowercase()
    };
    items.sort_by(|a,b| {
        let primary = match sort {
            SortBy::NameAsc => lower_name(*a).cmp(&lower_name(*b)),
            SortBy::NameDesc => lower_name(*b).cmp(&lower_name(*a)),
            SortBy::HighestRated =>
                rating(stars, &paths[*b]).cmp(&rating(stars, &paths[*a])),
            SortBy::Folder => paths[*a].parent().cmp(&paths[*b].parent()),
            SortBy::Queue => Ordering::Equal,
        };
        primary.then_with(|| lower_name(*a).cmp(&lower_name(*b)))
            .then_with(|| a.cmp(b))
    });
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    fn paths() -> Vec<PathBuf> {
        vec!["D:/Music/zebra.mp3".into(), "D:/Other/alpha.flac".into(),
             "D:/Music/beta.mp3".into()]
    }
    #[test]
    fn rank_and_name_sort_keep_underlying_queue_indices() {
        let songs = paths();
        let mut stars = HashMap::new();
        next_rating(&mut stars, songs[2].clone());
        next_rating(&mut stars, songs[2].clone());
        assert_eq!(filtered_order(&songs,&stars,"",SortBy::HighestRated),vec![2,1,0]);
        assert_eq!(filtered_order(&songs,&stars,"",SortBy::NameAsc),vec![1,2,0]);
        assert_eq!(filtered_order(&songs,&stars,"",SortBy::Queue),vec![0,1,2]);
    }
    #[test]
    fn filter_is_case_insensitive_and_rating_cycles_to_zero() {
        let songs=paths();
        let mut stars=HashMap::new();
        assert_eq!(filtered_order(&songs,&stars,"ALPHA",SortBy::Queue),vec![1]);
        for level in 1..=5 { assert_eq!(next_rating(&mut stars,songs[1].clone()),level); }
        assert_eq!(next_rating(&mut stars,songs[1].clone()),0);
        assert!(!stars.contains_key(&songs[1]));
    }
}
