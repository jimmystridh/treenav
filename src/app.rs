use crate::{config::Config, size::SizeWorker, state::PersistentState, tree, ui};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{layout::Rect, prelude::*};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Duration;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use tui_tree_widget::{TreeItem, TreeState};

fn fuzzy_score(haystack: &str, needle: &[char]) -> Option<u16> {
    if needle.is_empty() {
        return Some(0);
    }
    let mut score: u16 = 0;
    let mut needle_idx = 0;
    let mut prev_match = false;
    let mut prev_was_separator = true;

    for c in haystack.chars() {
        let is_separator = c == '/' || c == '.' || c == '_' || c == '-' || c == ' ';
        if needle_idx < needle.len() && c == needle[needle_idx] {
            score += if prev_was_separator { 10 } else if prev_match { 5 } else { 1 };
            needle_idx += 1;
            prev_match = true;
        } else {
            prev_match = false;
        }
        prev_was_separator = is_separator;
    }

    if needle_idx == needle.len() {
        Some(score)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Tree,
    Starred,
    Bookmarks,
    Recent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Search,
    BookmarkLabel,
}

pub struct App {
    pub tree_state: TreeState<PathBuf>,
    pub items: Vec<TreeItem<'static, PathBuf>>,
    pub root_path: PathBuf,
    pub persistent_state: PersistentState,
    pub config: Config,
    pub should_quit: bool,
    pub visible_height: u16,
    pub selected_dir: Option<PathBuf>,
    pub view_mode: ViewMode,
    pub input_mode: InputMode,
    pub show_help: bool,
    pub tree_area: Rect,
    pub search_input: Input,
    pub search_matches: Vec<(PathBuf, u16)>,
    pub search_index: usize,
    search_paths_cache: Vec<PathBuf>,
    pub show_preview: bool,
    pub bookmark_input: Input,
    pub bookmark_path: Option<PathBuf>,
    pub dir_sizes: HashMap<PathBuf, Option<u64>>,
    size_worker: SizeWorker,
    saved_view_items: Option<Vec<TreeItem<'static, PathBuf>>>,
    saved_selection: Option<Vec<PathBuf>>,
    last_click_time: std::time::Instant,
    last_click_row: u16,
}

impl App {
    pub fn new(path: PathBuf) -> Result<Self> {
        let persistent_state = PersistentState::load();
        let config = Config::load();
        let items = tree::build_tree(
            &path,
            &persistent_state.expanded_dirs,
            &persistent_state.starred_dirs,
            persistent_state.show_hidden,
            None,
        )?;

        let mut tree_state = TreeState::default();
        for expanded in &persistent_state.expanded_dirs {
            tree_state.open(vec![expanded.clone()]);
        }
        tree_state.select_first();

        Ok(Self {
            tree_state,
            items,
            root_path: path,
            persistent_state,
            config,
            should_quit: false,
            visible_height: 20,
            selected_dir: None,
            view_mode: ViewMode::Tree,
            input_mode: InputMode::Normal,
            show_help: false,
            tree_area: Rect::default(),
            search_input: Input::default(),
            search_matches: Vec::new(),
            search_index: 0,
            search_paths_cache: Vec::new(),
            show_preview: false,
            bookmark_input: Input::default(),
            bookmark_path: None,
            dir_sizes: HashMap::new(),
            size_worker: SizeWorker::new(),
            saved_view_items: None,
            saved_selection: None,
            last_click_time: std::time::Instant::now(),
            last_click_row: 0,
        })
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<BufWriter<File>>>) -> Result<()> {
        while !self.should_quit {
            // Poll for size calculation results
            self.size_worker.poll_results(&mut self.dir_sizes);

            terminal.draw(|frame| ui::render(frame, self))?;

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key),
                    Event::Mouse(mouse) => self.handle_mouse(mouse),
                    _ => {}
                }
            }
        }

        self.persistent_state.save()?;
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // If help is showing, any key closes it
        if self.show_help {
            self.show_help = false;
            return;
        }

        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Search => self.handle_search_key(key),
            InputMode::BookmarkLabel => self.handle_bookmark_label_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => self.should_quit = true,
            (_, KeyCode::Char('q')) => self.should_quit = true,
            (_, KeyCode::Esc) => self.should_quit = true,

            // Help
            (KeyModifiers::SHIFT, KeyCode::Char('?')) | (_, KeyCode::Char('?')) => {
                self.show_help = true;
            }

            // Search
            (_, KeyCode::Char('/')) => {
                self.enter_search_mode();
            }

            (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
                self.tree_state.key_up();
            }
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
                self.tree_state.key_down();
            }
            (_, KeyCode::Left) | (_, KeyCode::Char('h')) => {
                if self.view_mode == ViewMode::Tree {
                    self.collapse_or_parent();
                }
            }
            (_, KeyCode::Right) | (_, KeyCode::Char('l')) => {
                if self.view_mode == ViewMode::Tree {
                    self.expand_selected();
                }
            }
            (_, KeyCode::Char(' ')) => {
                if self.view_mode == ViewMode::Tree {
                    self.toggle_selected();
                }
            }
            (_, KeyCode::Enter) => {
                self.select_and_quit();
            }

            // Star toggle
            (_, KeyCode::Char('s')) => {
                self.toggle_star();
            }
            // Switch view mode
            (KeyModifiers::SHIFT, KeyCode::Char('S')) => {
                self.toggle_view_mode();
            }

            (_, KeyCode::PageUp) => self.page_up(),
            (_, KeyCode::PageDown) => self.page_down(),
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => self.half_page_up(),
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => self.half_page_down(),

            (_, KeyCode::Home) | (_, KeyCode::Char('g')) => {
                self.tree_state.select_first();
            }
            (_, KeyCode::End) | (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                self.tree_state.select_last();
            }

            // Toggle hidden files
            (_, KeyCode::Char('.')) => {
                self.toggle_hidden();
            }

            // Toggle preview pane
            (_, KeyCode::Char('p')) => {
                self.show_preview = !self.show_preview;
            }

            // Bookmarks
            (_, KeyCode::Char('b')) => {
                self.add_or_edit_bookmark();
            }
            (KeyModifiers::SHIFT, KeyCode::Char('B')) => {
                self.switch_to_bookmarks_view();
            }

            // Recent directories
            (_, KeyCode::Char('r')) => {
                self.switch_to_recent_view();
            }

            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.exit_search_mode();
            }
            KeyCode::Enter => {
                self.jump_to_search_result();
            }
            KeyCode::Down | KeyCode::Tab => {
                if !self.search_matches.is_empty() {
                    self.search_index = (self.search_index + 1) % self.search_matches.len();
                    self.select_search_match();
                }
            }
            KeyCode::Up | KeyCode::BackTab => {
                if !self.search_matches.is_empty() {
                    self.search_index = self.search_index.checked_sub(1).unwrap_or(self.search_matches.len() - 1);
                    self.select_search_match();
                }
            }
            _ => {
                let crossterm_event = crossterm::event::Event::Key(crossterm::event::KeyEvent {
                    code: key.code,
                    modifiers: key.modifiers,
                    kind: crossterm::event::KeyEventKind::Press,
                    state: crossterm::event::KeyEventState::NONE,
                });
                self.search_input.handle_event(&crossterm_event);
                self.update_search_matches();
                self.select_search_match();
            }
        }
    }

    fn select_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        // In filtered view, items are flat - just select by path
        let (path, _) = &self.search_matches[self.search_index];
        self.tree_state.select(vec![path.clone()]);
    }

    fn handle_bookmark_label_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.bookmark_input = Input::default();
                self.bookmark_path = None;
            }
            KeyCode::Enter => {
                if let Some(path) = self.bookmark_path.take() {
                    let label = self.bookmark_input.value().to_string();
                    self.persistent_state.add_bookmark(path, label);
                    self.rebuild_tree();
                }
                self.input_mode = InputMode::Normal;
                self.bookmark_input = Input::default();
            }
            _ => {
                let crossterm_event = crossterm::event::Event::Key(crossterm::event::KeyEvent {
                    code: key.code,
                    modifiers: key.modifiers,
                    kind: crossterm::event::KeyEventKind::Press,
                    state: crossterm::event::KeyEventState::NONE,
                });
                self.bookmark_input.handle_event(&crossterm_event);
            }
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.show_help {
            self.show_help = false;
            return;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let row = mouse.row;
                let col = mouse.column;

                // Check if click is within the tree area (excluding border)
                if col > self.tree_area.x
                    && col < self.tree_area.x + self.tree_area.width - 1
                    && row > self.tree_area.y
                    && row < self.tree_area.y + self.tree_area.height - 1
                {
                    let clicked_row = row - self.tree_area.y - 1;
                    let now = std::time::Instant::now();
                    let is_double_click = now.duration_since(self.last_click_time).as_millis() < 400
                        && clicked_row == self.last_click_row;

                    self.last_click_time = now;
                    self.last_click_row = clicked_row;

                    // Select the clicked item using click_at
                    self.tree_state.click_at((col, row).into());

                    if is_double_click && self.view_mode == ViewMode::Tree {
                        self.toggle_selected();
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                for _ in 0..3 {
                    self.tree_state.key_up();
                }
            }
            MouseEventKind::ScrollDown => {
                for _ in 0..3 {
                    self.tree_state.key_down();
                }
            }
            _ => {}
        }
    }

    fn get_selected_path(&self) -> Option<PathBuf> {
        self.tree_state.selected().last().cloned()
    }

    fn toggle_selected(&mut self) {
        if let Some(selected) = self.get_selected_path() {
            if selected.is_dir() {
                if self.persistent_state.expanded_dirs.contains(&selected) {
                    self.persistent_state.expanded_dirs.remove(&selected);
                    self.tree_state.close(&[selected]);
                } else {
                    self.persistent_state.expanded_dirs.insert(selected.clone());
                    self.tree_state.open(vec![selected.clone()]);
                    self.request_size_for_dir(&selected);
                }
                self.rebuild_tree();
            }
        }
    }

    fn toggle_star(&mut self) {
        if let Some(selected) = self.get_selected_path() {
            if selected.is_dir() {
                if self.persistent_state.starred_dirs.contains(&selected) {
                    self.persistent_state.starred_dirs.remove(&selected);
                } else {
                    self.persistent_state.starred_dirs.insert(selected);
                }
                self.rebuild_tree();
            }
        }
    }

    fn toggle_view_mode(&mut self) {
        match self.view_mode {
            ViewMode::Tree => {
                // Save current items before switching to starred
                self.saved_view_items = Some(std::mem::take(&mut self.items));
                self.saved_selection = Some(self.tree_state.selected().to_vec());
                self.view_mode = ViewMode::Starred;
                self.rebuild_tree();
                self.tree_state = TreeState::default();
                self.tree_state.select_first();
            }
            ViewMode::Starred | ViewMode::Bookmarks | ViewMode::Recent => {
                // Restore saved tree
                self.view_mode = ViewMode::Tree;
                if let Some(items) = self.saved_view_items.take() {
                    self.items = items;
                    self.rebuild_tree();
                    self.tree_state = TreeState::default();
                    if let Some(sel) = self.saved_selection.take() {
                        self.tree_state.select(sel);
                    } else {
                        self.tree_state.select_first();
                    }
                } else {
                    self.rebuild_tree();
                    self.tree_state.select_first();
                }
            }
        }
    }

    fn select_and_quit(&mut self) {
        if let Some(selected) = self.get_selected_path() {
            if selected.is_dir() {
                self.persistent_state.add_recent(selected.clone());
                self.selected_dir = Some(selected);
                self.should_quit = true;
            }
        }
    }

    fn expand_selected(&mut self) {
        if let Some(selected) = self.get_selected_path() {
            if selected.is_dir() && !self.persistent_state.expanded_dirs.contains(&selected) {
                self.persistent_state.expanded_dirs.insert(selected.clone());
                self.tree_state.open(vec![selected.clone()]);
                self.rebuild_tree();
                self.request_size_for_dir(&selected);
            }
        }
    }

    fn request_size_for_dir(&mut self, path: &PathBuf) {
        if !self.dir_sizes.contains_key(path) {
            self.dir_sizes.insert(path.clone(), None);
            self.size_worker.request_size(path.clone());
        }
    }

    fn collapse_or_parent(&mut self) {
        if let Some(selected) = self.get_selected_path() {
            if selected.is_dir() && self.persistent_state.expanded_dirs.contains(&selected) {
                self.persistent_state.expanded_dirs.remove(&selected);
                self.tree_state.close(&[selected]);
                self.rebuild_tree();
            } else {
                self.tree_state.key_left();
            }
        } else {
            self.tree_state.key_left();
        }
    }

    fn toggle_hidden(&mut self) {
        self.persistent_state.show_hidden = !self.persistent_state.show_hidden;
        self.rebuild_tree();
    }

    fn rebuild_tree(&mut self) {
        let items = match self.view_mode {
            ViewMode::Tree => tree::build_tree(
                &self.root_path,
                &self.persistent_state.expanded_dirs,
                &self.persistent_state.starred_dirs,
                self.persistent_state.show_hidden,
                Some(&self.dir_sizes),
            ),
            ViewMode::Starred => tree::build_starred_list(&self.persistent_state.starred_dirs),
            ViewMode::Bookmarks => tree::build_bookmarks_list(&self.persistent_state.bookmarks),
            ViewMode::Recent => tree::build_recent_list(&self.persistent_state.recent_dirs),
        };
        if let Ok(items) = items {
            self.items = items;
        }
    }

    fn page_up(&mut self) {
        for _ in 0..self.visible_height {
            self.tree_state.key_up();
        }
    }

    fn page_down(&mut self) {
        for _ in 0..self.visible_height {
            self.tree_state.key_down();
        }
    }

    fn half_page_up(&mut self) {
        for _ in 0..(self.visible_height / 2) {
            self.tree_state.key_up();
        }
    }

    fn half_page_down(&mut self) {
        for _ in 0..(self.visible_height / 2) {
            self.tree_state.key_down();
        }
    }

    fn enter_search_mode(&mut self) {
        self.input_mode = InputMode::Search;
        self.search_input = Input::default();
        self.search_matches.clear();
        self.search_index = 0;
        // Save current selection and items
        self.saved_selection = Some(self.tree_state.selected().to_vec());
        self.saved_view_items = Some(self.items.clone());
        // Cache paths from already-loaded tree items (no disk I/O)
        self.search_paths_cache = self.collect_paths_from_items();
    }

    fn collect_paths_from_items(&self) -> Vec<PathBuf> {
        fn collect_recursive(items: &[TreeItem<'static, PathBuf>], paths: &mut Vec<PathBuf>) {
            for item in items {
                paths.push(item.identifier().clone());
                collect_recursive(item.children(), paths);
            }
        }
        let mut paths = Vec::new();
        collect_recursive(&self.items, &mut paths);
        paths
    }

    fn exit_search_mode(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_input = Input::default();
        self.search_matches.clear();
        self.search_index = 0;
        self.search_paths_cache.clear();
        // Restore original tree
        if let Some(items) = self.saved_view_items.take() {
            self.items = items;
            self.tree_state = TreeState::default();
            if let Some(sel) = self.saved_selection.take() {
                self.tree_state.select(sel);
            } else {
                self.tree_state.select_first();
            }
        }
    }

    fn update_search_matches(&mut self) {
        let query = self.search_input.value();
        if query.is_empty() {
            self.search_matches.clear();
            self.search_index = 0;
            // Restore original tree when query is cleared
            if let Some(items) = &self.saved_view_items {
                self.items = items.clone();
            }
            self.tree_state = TreeState::default();
            self.tree_state.select_first();
            return;
        }

        let query_lower = query.to_lowercase();
        let query_chars: Vec<char> = query_lower.chars().collect();

        let mut matches: Vec<(PathBuf, u16)> = self.search_paths_cache
            .iter()
            .filter_map(|path| {
                let name = path.file_name()?.to_string_lossy().to_lowercase();
                fuzzy_score(&name, &query_chars).map(|score| (path.clone(), score))
            })
            .collect();

        matches.sort_by(|a, b| b.1.cmp(&a.1));
        matches.truncate(50);

        // Build filtered tree from matches
        self.items = matches
            .iter()
            .map(|(path, _)| {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                let icon = crate::icons::get_icon(path, false);
                let display = format!("{} {}", icon, name);
                TreeItem::new_leaf(path.clone(), display)
            })
            .collect();

        self.search_matches = matches;
        self.search_index = 0;
        self.tree_state = TreeState::default();
        self.tree_state.select_first();
    }

    fn jump_to_search_result(&mut self) {
        if self.search_matches.is_empty() {
            self.exit_search_mode();
            return;
        }

        let (path, _) = &self.search_matches[self.search_index];
        let path = path.clone();

        // Restore original tree first
        if let Some(items) = self.saved_view_items.take() {
            self.items = items;
        }
        self.saved_selection = None;

        // Build full selection path from root to target
        let mut selection_path: Vec<PathBuf> = Vec::new();
        let mut current = Some(path.as_path());

        while let Some(p) = current {
            if p.starts_with(&self.root_path) && p != self.root_path {
                selection_path.push(p.to_path_buf());
            }
            current = p.parent();
        }
        selection_path.reverse();

        // Expand all parent directories
        for ancestor in &selection_path[..selection_path.len().saturating_sub(1)] {
            if !self.persistent_state.expanded_dirs.contains(ancestor) {
                self.persistent_state.expanded_dirs.insert(ancestor.clone());
            }
        }

        self.rebuild_tree();
        self.tree_state = TreeState::default();
        // Open expanded dirs in tree state
        for expanded in &self.persistent_state.expanded_dirs {
            self.tree_state.open(vec![expanded.clone()]);
        }
        self.tree_state.select(selection_path);

        // Clear search state
        self.input_mode = InputMode::Normal;
        self.search_input = Input::default();
        self.search_matches.clear();
        self.search_index = 0;
        self.search_paths_cache.clear();
    }

    fn add_or_edit_bookmark(&mut self) {
        if let Some(selected) = self.get_selected_path() {
            if selected.is_dir() {
                let existing_label = self.persistent_state
                    .get_bookmark(&selected)
                    .map(|b| b.label.clone())
                    .unwrap_or_default();

                self.bookmark_path = Some(selected);
                self.bookmark_input = Input::default().with_value(existing_label);
                self.input_mode = InputMode::BookmarkLabel;
            }
        }
    }

    fn switch_to_bookmarks_view(&mut self) {
        match self.view_mode {
            ViewMode::Bookmarks => {
                // Return to tree view
                self.view_mode = ViewMode::Tree;
                if let Some(items) = self.saved_view_items.take() {
                    self.items = items;
                    self.rebuild_tree();
                    self.tree_state = TreeState::default();
                    if let Some(sel) = self.saved_selection.take() {
                        self.tree_state.select(sel);
                    } else {
                        self.tree_state.select_first();
                    }
                } else {
                    self.rebuild_tree();
                    self.tree_state.select_first();
                }
            }
            _ => {
                // Save current state and switch to bookmarks
                self.saved_selection = Some(self.tree_state.selected().to_vec());
                self.saved_view_items = Some(std::mem::take(&mut self.items));
                self.view_mode = ViewMode::Bookmarks;
                self.rebuild_tree();
                self.tree_state = TreeState::default();
                self.tree_state.select_first();
            }
        }
    }

    fn switch_to_recent_view(&mut self) {
        match self.view_mode {
            ViewMode::Recent => {
                // Return to tree view
                self.view_mode = ViewMode::Tree;
                if let Some(items) = self.saved_view_items.take() {
                    self.items = items;
                    self.rebuild_tree();
                    self.tree_state = TreeState::default();
                    if let Some(sel) = self.saved_selection.take() {
                        self.tree_state.select(sel);
                    } else {
                        self.tree_state.select_first();
                    }
                } else {
                    self.rebuild_tree();
                    self.tree_state.select_first();
                }
            }
            _ => {
                // Save current state and switch to recent
                self.saved_selection = Some(self.tree_state.selected().to_vec());
                self.saved_view_items = Some(std::mem::take(&mut self.items));
                self.view_mode = ViewMode::Recent;
                self.rebuild_tree();
                self.tree_state = TreeState::default();
                self.tree_state.select_first();
            }
        }
    }
}
