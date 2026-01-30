use crate::icons;
use crate::size;
use crate::state::Bookmark;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tui_tree_widget::TreeItem;

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}

pub fn format_entry_name(
    path: &Path,
    is_expanded: bool,
    is_starred: bool,
    dir_sizes: Option<&HashMap<PathBuf, Option<u64>>>,
) -> String {
    let icon = icons::get_icon(path, is_expanded);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    let star = if is_starred { " ★" } else { "" };

    let size_str = if is_expanded && path.is_dir() {
        dir_sizes
            .and_then(|sizes| sizes.get(&path.to_path_buf()))
            .map(|size| match size {
                Some(bytes) => format!(" [{}]", size::format_size(*bytes)),
                None => " [...]".to_string(),
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    format!("{} {}{}{}", icon, name, star, size_str)
}

fn sort_entries(entries: &mut [PathBuf]) {
    entries.sort_by(|a, b| {
        let a_is_dir = a.is_dir();
        let b_is_dir = b.is_dir();
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let a_name = a.file_name().map(|n| n.to_ascii_lowercase());
                let b_name = b.file_name().map(|n| n.to_ascii_lowercase());
                a_name.cmp(&b_name)
            }
        }
    });
}

pub fn build_tree_item(
    path: &Path,
    expanded_dirs: &HashSet<PathBuf>,
    starred_dirs: &HashSet<PathBuf>,
    show_hidden: bool,
    dir_sizes: Option<&HashMap<PathBuf, Option<u64>>>,
) -> io::Result<TreeItem<'static, PathBuf>> {
    let is_expanded = expanded_dirs.contains(path);
    let is_starred = starred_dirs.contains(path);
    let name = format_entry_name(path, is_expanded, is_starred, dir_sizes);

    if path.is_dir() && is_expanded {
        match load_children(path, expanded_dirs, starred_dirs, show_hidden, dir_sizes) {
            Ok(children) => TreeItem::new(path.to_path_buf(), name, children)
                .map_err(|e| io::Error::other(format!("Tree item error: {}", e))),
            Err(e) => {
                let error_name = format!("{} [{}]", name, format_error(&e));
                Ok(TreeItem::new_leaf(path.to_path_buf(), error_name))
            }
        }
    } else {
        Ok(TreeItem::new_leaf(path.to_path_buf(), name))
    }
}

fn load_children(
    dir: &Path,
    expanded_dirs: &HashSet<PathBuf>,
    starred_dirs: &HashSet<PathBuf>,
    show_hidden: bool,
    dir_sizes: Option<&HashMap<PathBuf, Option<u64>>>,
) -> io::Result<Vec<TreeItem<'static, PathBuf>>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| show_hidden || !is_hidden(p))
        .collect();

    sort_entries(&mut entries);

    let children: Vec<TreeItem<'static, PathBuf>> = entries
        .iter()
        .filter_map(|p| {
            build_tree_item(p, expanded_dirs, starred_dirs, show_hidden, dir_sizes).ok()
        })
        .collect();

    Ok(children)
}

fn format_error(e: &io::Error) -> &'static str {
    match e.kind() {
        io::ErrorKind::PermissionDenied => "Permission denied",
        io::ErrorKind::NotFound => "Not found",
        _ => "Error",
    }
}

pub fn build_tree(
    root: &Path,
    expanded_dirs: &HashSet<PathBuf>,
    starred_dirs: &HashSet<PathBuf>,
    show_hidden: bool,
    dir_sizes: Option<&HashMap<PathBuf, Option<u64>>>,
) -> io::Result<Vec<TreeItem<'static, PathBuf>>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| show_hidden || !is_hidden(p))
        .collect();

    sort_entries(&mut entries);

    let items: Vec<TreeItem<'static, PathBuf>> = entries
        .iter()
        .filter_map(|p| {
            build_tree_item(p, expanded_dirs, starred_dirs, show_hidden, dir_sizes).ok()
        })
        .collect();

    Ok(items)
}

pub fn build_starred_list(
    starred_dirs: &HashSet<PathBuf>,
) -> io::Result<Vec<TreeItem<'static, PathBuf>>> {
    let mut dirs: Vec<PathBuf> = starred_dirs.iter().cloned().collect();
    dirs.sort_by(|a, b| {
        let a_name = a.file_name().map(|n| n.to_ascii_lowercase());
        let b_name = b.file_name().map(|n| n.to_ascii_lowercase());
        a_name.cmp(&b_name)
    });

    let items: Vec<TreeItem<'static, PathBuf>> = dirs
        .into_iter()
        .filter(|p| p.exists())
        .map(|p| {
            let name = format!("★ {}", p.display());
            TreeItem::new_leaf(p, name)
        })
        .collect();

    Ok(items)
}

pub fn build_bookmarks_list(bookmarks: &[Bookmark]) -> io::Result<Vec<TreeItem<'static, PathBuf>>> {
    let items: Vec<TreeItem<'static, PathBuf>> = bookmarks
        .iter()
        .filter(|b| b.path.exists())
        .map(|b| {
            let display = if b.label.is_empty() {
                format!("📌 {}", b.path.display())
            } else {
                format!(
                    "📌 {} ({})",
                    b.label,
                    b.path.file_name().unwrap_or_default().to_string_lossy()
                )
            };
            TreeItem::new_leaf(b.path.clone(), display)
        })
        .collect();

    Ok(items)
}

pub fn build_recent_list(
    recent_dirs: &VecDeque<PathBuf>,
) -> io::Result<Vec<TreeItem<'static, PathBuf>>> {
    let items: Vec<TreeItem<'static, PathBuf>> = recent_dirs
        .iter()
        .filter(|p| p.exists())
        .map(|p| {
            let name = format!("⏱ {}", p.display());
            TreeItem::new_leaf(p.clone(), name)
        })
        .collect();

    Ok(items)
}
