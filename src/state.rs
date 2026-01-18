use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub path: PathBuf,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub created_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistentState {
    pub expanded_dirs: HashSet<PathBuf>,
    #[serde(default)]
    pub starred_dirs: HashSet<PathBuf>,
    #[serde(default)]
    pub show_hidden: bool,
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
    #[serde(default)]
    pub recent_dirs: VecDeque<PathBuf>,
}

impl PersistentState {
    fn state_file_path() -> Option<PathBuf> {
        dirs::data_dir().map(|p| p.join("treenav").join("state.json"))
    }

    pub fn load() -> Self {
        Self::state_file_path()
            .and_then(|path| fs::read_to_string(&path).ok())
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        if let Some(path) = Self::state_file_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            fs::write(&path, json)?;
        }
        Ok(())
    }

    pub fn add_bookmark(&mut self, path: PathBuf, label: String) {
        self.bookmarks.retain(|b| b.path != path);
        self.bookmarks.push(Bookmark {
            path,
            label,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        });
    }

    pub fn get_bookmark(&self, path: &PathBuf) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| &b.path == path)
    }

    pub fn add_recent(&mut self, path: PathBuf) {
        self.recent_dirs.retain(|p| p != &path);
        self.recent_dirs.push_front(path);
        while self.recent_dirs.len() > 50 {
            self.recent_dirs.pop_back();
        }
    }
}
