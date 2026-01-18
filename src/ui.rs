use crate::app::{App, InputMode, ViewMode};
use crate::config::Theme;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::fs;
use std::io::{BufRead, BufReader};
use tui_tree_widget::Tree;

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(frame.area());

    let main_area = chunks[0];
    let footer_area = chunks[1];

    if app.show_preview {
        let split = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(main_area);

        let tree_area = split[0];
        let preview_area = split[1];

        app.visible_height = tree_area.height.saturating_sub(2);
        app.tree_area = tree_area;

        render_tree(frame, app, tree_area);
        render_preview(frame, app, preview_area);
    } else {
        app.visible_height = main_area.height.saturating_sub(2);
        app.tree_area = main_area;

        render_tree(frame, app, main_area);
    }

    if app.input_mode == InputMode::Search {
        render_search_bar(frame, app, footer_area);
    } else {
        render_footer(frame, app, footer_area);
    }

    if app.show_help {
        render_help(frame, &app.config.theme);
    }

    if app.input_mode == InputMode::BookmarkLabel {
        render_bookmark_input(frame, app);
    }
}

fn render_tree(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.config.theme;

    let title = match app.view_mode {
        ViewMode::Tree => format!(" {} ", app.root_path.display()),
        ViewMode::Starred => " ★ Starred ".to_string(),
        ViewMode::Bookmarks => " 📌 Bookmarks ".to_string(),
        ViewMode::Recent => " ⏱ Recent ".to_string(),
    };

    let title_style = match app.view_mode {
        ViewMode::Tree => Style::default().fg(theme.border).add_modifier(Modifier::BOLD),
        ViewMode::Starred | ViewMode::Bookmarks | ViewMode::Recent => {
            Style::default().fg(theme.starred).add_modifier(Modifier::BOLD)
        }
    };

    let border_color = match app.view_mode {
        ViewMode::Tree => theme.border,
        ViewMode::Starred | ViewMode::Bookmarks | ViewMode::Recent => theme.starred,
    };

    let block = tui_tree_widget::Block::bordered()
        .title(title)
        .title_style(title_style)
        .border_style(Style::default().fg(border_color));

    let highlight_style = Style::default()
        .bg(theme.highlight_bg)
        .fg(theme.text)
        .add_modifier(Modifier::BOLD);

    let tree = Tree::new(&app.items)
        .expect("Tree items should be valid")
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(tree, area, &mut app.tree_state);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mut keys = match app.view_mode {
        ViewMode::Tree => vec![
            ("↑↓/jk", "nav"),
            ("←→/hl", "tree"),
            ("Space", "toggle"),
            ("Enter", "cd"),
            ("s", "star"),
            ("b", "mark"),
            ("/", "search"),
            ("p", "preview"),
            (".", "hidden"),
            ("B", "marks"),
            ("r", "recent"),
            ("?", "help"),
            ("q", "quit"),
        ],
        ViewMode::Starred => vec![
            ("↑↓/jk", "navigate"),
            ("Enter", "cd"),
            ("s", "unstar"),
            ("S", "back"),
            ("?", "help"),
            ("q", "quit"),
        ],
        ViewMode::Bookmarks => vec![
            ("↑↓/jk", "navigate"),
            ("Enter", "cd"),
            ("B", "back"),
            ("?", "help"),
            ("q", "quit"),
        ],
        ViewMode::Recent => vec![
            ("↑↓/jk", "navigate"),
            ("Enter", "cd"),
            ("r", "back"),
            ("?", "help"),
            ("q", "quit"),
        ],
    };

    if app.persistent_state.show_hidden && app.view_mode == ViewMode::Tree {
        keys.insert(0, ("●", "hidden"));
    }

    let theme = &app.config.theme;
    let spans: Vec<Span> = keys
        .iter()
        .enumerate()
        .flat_map(|(i, (key, desc))| {
            let mut v = vec![
                Span::styled(*key, Style::default().fg(theme.border).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", desc), Style::default().fg(theme.dim)),
            ];
            if i < keys.len() - 1 {
                v.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
            }
            v
        })
        .collect();

    let footer = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    frame.render_widget(footer, area);
}

fn render_help(frame: &mut Frame, theme: &Theme) {
    let area = frame.area();

    // Center the help popup
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 22.min(area.height.saturating_sub(4));
    let popup_area = Rect {
        x: (area.width - popup_width) / 2,
        y: (area.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(vec![
            Span::styled("  treenav", Style::default().fg(theme.border).add_modifier(Modifier::BOLD)),
            Span::styled(" - Terminal Directory Navigator", Style::default().fg(theme.dim)),
        ]),
        Line::from(""),
        Line::styled("  NAVIGATION", Style::default().fg(theme.starred).add_modifier(Modifier::BOLD)),
        help_line("↑ / k", "Move up", theme),
        help_line("↓ / j", "Move down", theme),
        help_line("← / h", "Collapse directory / go to parent", theme),
        help_line("→ / l", "Expand directory", theme),
        help_line("Space", "Toggle expand/collapse", theme),
        help_line("g / Home", "Go to first item", theme),
        help_line("G / End", "Go to last item", theme),
        help_line("PgUp/PgDn", "Page up/down", theme),
        help_line("Ctrl+u/d", "Half page up/down", theme),
        Line::from(""),
        Line::styled("  ACTIONS", Style::default().fg(theme.starred).add_modifier(Modifier::BOLD)),
        help_line("Enter", "cd to selected directory and exit", theme),
        help_line("s", "Toggle star on directory", theme),
        help_line("S", "Switch to/from starred view", theme),
        help_line("/", "Fuzzy search files and folders", theme),
        help_line("p", "Toggle preview pane", theme),
        help_line(".", "Toggle hidden files", theme),
        help_line("b", "Add/edit bookmark with label", theme),
        help_line("B", "Open/close bookmarks view", theme),
        help_line("r", "Open/close recent directories", theme),
        help_line("q / Ctrl+c", "Quit without changing directory", theme),
        help_line("?", "Toggle this help", theme),
        Line::from(""),
        Line::styled("  Press any key to close", Style::default().fg(theme.dim).add_modifier(Modifier::ITALIC)),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(" Help ")
                .title_style(Style::default().fg(theme.border).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(Color::Rgb(15, 15, 25))),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(help, popup_area);
}

fn help_line<'a>(key: &'a str, desc: &'a str, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:14}", key), Style::default().fg(theme.border)),
        Span::styled(desc, Style::default().fg(theme.text)),
    ])
}

fn render_preview(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.config.theme;
    let selected = app.tree_state.selected().last().cloned();

    let (title, content) = match selected {
        Some(ref path) if path.is_dir() => {
            let title = format!(" {} ", path.file_name().unwrap_or_default().to_string_lossy());
            let entries: Vec<String> = fs::read_dir(path)
                .map(|rd| {
                    let mut items: Vec<String> = rd
                        .filter_map(|e| e.ok())
                        .filter(|e| app.persistent_state.show_hidden || !e.file_name().to_string_lossy().starts_with('.'))
                        .map(|e| {
                            let name = e.file_name().to_string_lossy().to_string();
                            if e.path().is_dir() {
                                format!("{}/", name)
                            } else {
                                name
                            }
                        })
                        .collect();
                    items.sort_by(|a, b| {
                        let a_is_dir = a.ends_with('/');
                        let b_is_dir = b.ends_with('/');
                        match (a_is_dir, b_is_dir) {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => a.to_lowercase().cmp(&b.to_lowercase()),
                        }
                    });
                    items
                })
                .unwrap_or_default();
            (title, entries.join("\n"))
        }
        Some(ref path) if path.is_file() => {
            let title = format!(" {} ", path.file_name().unwrap_or_default().to_string_lossy());
            let content = fs::File::open(path)
                .ok()
                .map(|f| {
                    BufReader::new(f)
                        .lines()
                        .take(100)
                        .filter_map(|l| l.ok())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_else(|| "[Unable to read file]".to_string());
            (title, content)
        }
        _ => (" Preview ".to_string(), "Select a file or directory".to_string()),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(title)
        .title_style(Style::default().fg(theme.border).add_modifier(Modifier::BOLD));

    let paragraph = Paragraph::new(content)
        .style(Style::default().fg(theme.dim))
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.config.theme;

    // Search input bar at bottom
    let input_width = area.width.saturating_sub(3) as usize;
    let scroll = app.search_input.visual_scroll(input_width);

    let search_text = format!("/{}", app.search_input.value());
    let count_text = if app.search_input.value().is_empty() {
        String::new()
    } else if app.search_matches.is_empty() {
        " [no match]".to_string()
    } else {
        format!(" [{}/{}]", app.search_index + 1, app.search_matches.len())
    };

    let line = Line::from(vec![
        Span::styled(&search_text, Style::default().fg(theme.text)),
        Span::styled(&count_text, Style::default().fg(theme.dim)),
    ]);

    let input = Paragraph::new(line)
        .style(Style::default().bg(Color::Rgb(30, 30, 40)));

    frame.render_widget(input, area);

    // Set cursor position
    let cursor_x = area.x + 1 + (app.search_input.visual_cursor().saturating_sub(scroll)) as u16;
    frame.set_cursor_position((cursor_x, area.y));
}

fn render_bookmark_input(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let theme = &app.config.theme;

    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 5;
    let popup_area = Rect {
        x: (area.width - popup_width) / 2,
        y: (area.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let path_name = app.bookmark_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let title = format!(" Bookmark: {} ", path_name);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.starred))
        .title(title)
        .title_style(Style::default().fg(theme.starred).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Rgb(15, 15, 25)));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);

    // Label prompt
    frame.render_widget(
        Paragraph::new("Label (optional):").style(Style::default().fg(theme.dim)),
        chunks[0],
    );

    // Input field
    let input_width = chunks[1].width.saturating_sub(1) as usize;
    let scroll = app.bookmark_input.visual_scroll(input_width);
    let input = Paragraph::new(app.bookmark_input.value())
        .style(Style::default().fg(theme.text))
        .scroll((0, scroll as u16));

    frame.render_widget(input, chunks[1]);

    let cursor_x = chunks[1].x + (app.bookmark_input.visual_cursor().saturating_sub(scroll)) as u16;
    frame.set_cursor_position((cursor_x, chunks[1].y));
}
