# treenav

A fast, beautiful terminal directory navigator with persistent state, fuzzy search, and vim-style keybindings.

![treenav](https://img.shields.io/badge/rust-1.70+-orange.svg)
![license](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- **Tree navigation** - Browse directories with vim-style or arrow keys
- **Fuzzy search** - Press `/` to filter and find files instantly
- **Persistent state** - Expanded directories, bookmarks, and recent locations are remembered
- **Bookmarks** - Save frequently used directories with custom labels
- **Recent directories** - Quick access to recently visited locations
- **Preview pane** - See directory contents or file previews side-by-side
- **Mouse support** - Click to select, scroll to navigate, double-click to expand
- **Custom themes** - Configure colors via `~/.config/treenav/config.toml`
- **Directory sizes** - See sizes of expanded directories (calculated in background)
- **Hidden files toggle** - Press `.` to show/hide dotfiles
- **Shell integration** - Press Enter to `cd` directly to the selected directory
- **Nerd Font icons** - Beautiful file type icons

## Installation

```bash
git clone https://github.com/yourusername/treenav
cd treenav
cargo build --release
cp target/release/treenav ~/.local/bin/
```

### Shell Integration

Add to your `~/.zshrc`:

```bash
source /path/to/treenav/treenav.zsh
```

This provides the `tn` command which runs treenav and `cd`s to the selected directory.

## Usage

```bash
tn              # Start in current directory
tn ~/projects   # Start in specific directory
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Collapse / go to parent |
| `l` / `→` | Expand directory |
| `Space` | Toggle expand/collapse |
| `g` / `Home` | First item |
| `G` / `End` | Last item |
| `Ctrl+d` / `PgDn` | Half page / page down |
| `Ctrl+u` / `PgUp` | Half page / page up |

### Actions

| Key | Action |
|-----|--------|
| `Enter` | cd to directory and exit |
| `/` | Fuzzy search (filters tree) |
| `p` | Toggle preview pane |
| `.` | Toggle hidden files |
| `s` | Star/unstar directory |
| `S` | Open starred view |
| `b` | Add/edit bookmark with label |
| `B` | Open bookmarks view |
| `r` | Open recent directories |
| `?` | Show help |
| `q` / `Esc` | Quit |

### Search Mode

| Key | Action |
|-----|--------|
| Type | Filter tree to matching items |
| `Enter` | Jump to selected match |
| `Tab` / `↓` | Next match |
| `Shift+Tab` / `↑` | Previous match |
| `Esc` | Cancel search |

### Mouse

| Action | Effect |
|--------|--------|
| Click | Select item |
| Double-click | Toggle expand |
| Scroll | Navigate up/down |

## Configuration

Create `~/.config/treenav/config.toml`:

```toml
[theme]
border = "#50C8DC"
highlight_bg = "#285064"
starred = "#FAC832"
text = "#E0E0E0"
dim = "#808080"
```

## State

Persistent state is stored at:
- **Linux**: `~/.local/share/treenav/state.json`
- **macOS**: `~/Library/Application Support/treenav/state.json`

Includes: expanded directories, starred directories, bookmarks, recent directories, hidden files preference.

## Requirements

- Terminal with true color support
- [Nerd Font](https://www.nerdfonts.com/) for icons (optional)

## License

MIT
