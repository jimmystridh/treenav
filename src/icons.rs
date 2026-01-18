use nerd_font_symbols::{cod, dev, fa, md};
use std::path::Path;

pub fn get_icon(path: &Path, is_expanded: bool) -> &'static str {
    if path.is_dir() {
        return get_dir_icon(is_expanded);
    }

    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => dev::DEV_RUST,
        Some("toml") => fa::FA_FILE_CODE,
        Some("json") => fa::FA_FILE_CODE,
        Some("md") => md::MD_LANGUAGE_MARKDOWN,
        Some("txt") => fa::FA_FILE_TEXT_O,
        Some("py") => dev::DEV_PYTHON,
        Some("js") => dev::DEV_JAVASCRIPT,
        Some("ts") => dev::DEV_JAVASCRIPT,
        Some("html") => dev::DEV_HTML5,
        Some("css") => dev::DEV_CSS3,
        Some("yml" | "yaml") => fa::FA_FILE_CODE,
        Some("sh" | "bash" | "zsh") => cod::COD_TERMINAL,
        Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "ico") => fa::FA_FILE_IMAGE,
        Some("zip" | "tar" | "gz" | "rar" | "7z") => fa::FA_FILE_ZIPPER,
        Some("pdf") => fa::FA_FILE_PDF,
        Some("mp3" | "wav" | "flac" | "ogg") => fa::FA_FILE_AUDIO,
        Some("mp4" | "avi" | "mkv" | "mov") => fa::FA_FILE_VIDEO,
        Some("lock") => fa::FA_LOCK,
        Some("git" | "gitignore") => dev::DEV_GIT,
        _ => fa::FA_FILE_O,
    }
}

pub fn get_dir_icon(is_expanded: bool) -> &'static str {
    if is_expanded {
        fa::FA_FOLDER_OPEN
    } else {
        fa::FA_FOLDER
    }
}
