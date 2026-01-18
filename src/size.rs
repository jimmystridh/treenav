use crossbeam_channel::{bounded, Receiver, Sender};
use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;
use walkdir::WalkDir;

pub struct SizeWorker {
    request_tx: Sender<PathBuf>,
    result_rx: Receiver<(PathBuf, u64)>,
}

impl SizeWorker {
    pub fn new() -> Self {
        let (request_tx, request_rx) = bounded::<PathBuf>(100);
        let (result_tx, result_rx) = bounded::<(PathBuf, u64)>(100);

        thread::spawn(move || {
            while let Ok(path) = request_rx.recv() {
                let size = calculate_dir_size(&path);
                let _ = result_tx.send((path, size));
            }
        });

        Self {
            request_tx,
            result_rx,
        }
    }

    pub fn request_size(&self, path: PathBuf) {
        let _ = self.request_tx.try_send(path);
    }

    pub fn poll_results(&self, sizes: &mut HashMap<PathBuf, Option<u64>>) {
        while let Ok((path, size)) = self.result_rx.try_recv() {
            sizes.insert(path, Some(size));
        }
    }
}

fn calculate_dir_size(path: &PathBuf) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}
