use ratatui::style::Color;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Theme {
    pub border: Color,
    pub highlight_bg: Color,
    pub starred: Color,
    pub dim: Color,
    pub text: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border: Color::Rgb(80, 200, 220),
            highlight_bg: Color::Rgb(40, 80, 100),
            starred: Color::Rgb(250, 200, 50),
            dim: Color::Rgb(100, 100, 100),
            text: Color::White,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub theme: Theme,
}

#[derive(Debug, Deserialize, Default)]
struct TomlConfig {
    #[serde(default)]
    theme: TomlTheme,
}

#[derive(Debug, Deserialize, Default)]
struct TomlTheme {
    border: Option<String>,
    highlight_bg: Option<String>,
    starred: Option<String>,
    dim: Option<String>,
    text: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_file_path();
        if let Some(path) = path {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(toml_config) = toml::from_str::<TomlConfig>(&contents) {
                    return Self::from_toml(toml_config);
                }
            }
        }
        Self::default()
    }

    fn config_file_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("treenav").join("config.toml"))
    }

    fn from_toml(toml: TomlConfig) -> Self {
        let default = Theme::default();
        Self {
            theme: Theme {
                border: toml.theme.border.as_deref().and_then(parse_color).unwrap_or(default.border),
                highlight_bg: toml.theme.highlight_bg.as_deref().and_then(parse_color).unwrap_or(default.highlight_bg),
                starred: toml.theme.starred.as_deref().and_then(parse_color).unwrap_or(default.starred),
                dim: toml.theme.dim.as_deref().and_then(parse_color).unwrap_or(default.dim),
                text: toml.theme.text.as_deref().and_then(parse_color).unwrap_or(default.text),
            },
        }
    }
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();

    // Handle hex colors like "#50C8DC" or "50C8DC"
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Some(Color::Rgb(r, g, b));
        }
    }

    // Handle named colors
    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => None,
    }
}
