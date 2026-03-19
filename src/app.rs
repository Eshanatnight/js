use core::time::Duration;

use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};

use crate::tree::JsonTree;
use crate::ui;

#[derive(PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

pub struct App {
    pub tree: JsonTree,
    pub visible: Vec<usize>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub input_mode: InputMode,
    pub search_query: String,
    pub search_results: Vec<usize>,
    pub search_index: usize,
    pub status_message: String,
    pub should_quit: bool,
    pub show_help: bool,
    pub filename: String,
}

impl App {
    pub fn new(tree: JsonTree, filename: String) -> Self {
        let visible = tree.visible_lines();
        Self {
            tree,
            visible,
            cursor: 0,
            scroll_offset: 0,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_results: Vec::new(),
            search_index: 0,
            status_message: String::new(),
            should_quit: false,
            show_help: false,
            filename,
        }
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| ui::draw(f, self))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match self.input_mode {
                    InputMode::Normal => self.handle_normal_key(key),
                    InputMode::Search => self.handle_search_key(key),
                },
                Event::Mouse(mouse) => self.handle_mouse(mouse),
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        if self.show_help {
            self.show_help = false;
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }

            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('g') | KeyCode::Home => self.go_top(),
            KeyCode::Char('G') | KeyCode::End => self.go_bottom(),
            KeyCode::PageUp | KeyCode::Char('b') => self.page_up(20),
            KeyCode::PageDown | KeyCode::Char('f') => self.page_down(20),

            KeyCode::Enter | KeyCode::Char(' ' | 'l') | KeyCode::Right => {
                self.toggle_or_expand();
            }
            KeyCode::Left | KeyCode::Char('h') => self.collapse_or_parent(),
            KeyCode::Char('e') => self.expand_all(),
            KeyCode::Char('c') => self.collapse_all(),

            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
                self.status_message = "/".to_string();
            }
            KeyCode::Char('n') => self.next_search_result(),
            KeyCode::Char('N') => self.prev_search_result(),

            KeyCode::Char('y') => self.copy_value(),
            KeyCode::Char('Y') => self.copy_path(),
            KeyCode::Char(d @ '1'..='9') => {
                self.expand_to_depth((d as u8 - b'0') as usize);
            }

            KeyCode::Char('?') => self.show_help = true,

            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                self.perform_search();
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search_query.clear();
                self.search_results.clear();
                self.status_message.clear();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.status_message = format!("/{}", self.search_query);
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.status_message = format!("/{}", self.search_query);
            }
            _ => {}
        }
    }

    fn refresh_visible(&mut self) {
        self.visible = self.tree.visible_lines();
        if self.cursor >= self.visible.len() {
            self.cursor = self.visible.len().saturating_sub(1);
        }
    }

    const fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    const fn move_down(&mut self) {
        if self.cursor + 1 < self.visible.len() {
            self.cursor += 1;
        }
    }

    const fn page_up(&mut self, amount: usize) {
        self.cursor = self.cursor.saturating_sub(amount);
    }

    fn page_down(&mut self, amount: usize) {
        self.cursor = (self.cursor + amount).min(self.visible.len().saturating_sub(1));
    }

    const fn go_top(&mut self) {
        self.cursor = 0;
    }

    const fn go_bottom(&mut self) {
        self.cursor = self.visible.len().saturating_sub(1);
    }

    fn toggle_or_expand(&mut self) {
        if let Some(&node_idx) = self.visible.get(self.cursor)
            && self.tree.nodes[node_idx].is_expandable()
        {
            self.tree.toggle(node_idx);
            self.refresh_visible();
        }
    }

    fn collapse_or_parent(&mut self) {
        if let Some(&node_idx) = self.visible.get(self.cursor) {
            let node = &self.tree.nodes[node_idx];
            if node.is_expandable() && node.expanded {
                self.tree.toggle(node_idx);
                self.refresh_visible();
            } else if node.depth > 0 {
                // Move to parent: find the nearest visible node with lower depth
                for i in (0..self.cursor).rev() {
                    if self.tree.nodes[self.visible[i]].depth < node.depth {
                        self.cursor = i;
                        break;
                    }
                }
            } else {
                // Node at root depth - already at top, no action
            }
        }
    }

    fn expand_all(&mut self) {
        self.tree.expand_all();
        self.refresh_visible();
        self.status_message = "Expanded all nodes".to_string();
    }

    fn collapse_all(&mut self) {
        self.tree.collapse_all();
        self.refresh_visible();
        self.status_message = "Collapsed all nodes".to_string();
    }

    pub fn current_path(&self) -> String {
        self.visible
            .get(self.cursor)
            .map_or_else(|| "$".to_string(), |&idx| self.tree.get_path(idx))
    }

    fn perform_search(&mut self) {
        self.search_results.clear();
        self.search_index = 0;

        if self.search_query.is_empty() {
            self.status_message.clear();
            return;
        }

        for (i, &node_idx) in self.visible.iter().enumerate() {
            if self.tree.node_matches(node_idx, &self.search_query) {
                self.search_results.push(i);
            }
        }

        if self.search_results.is_empty() {
            self.status_message = format!("Pattern not found: {query}", query = self.search_query);
        } else {
            self.search_index = self
                .search_results
                .iter()
                .position(|&r| r >= self.cursor)
                .unwrap_or(0);
            self.cursor = self.search_results[self.search_index];
            let n = self.search_index + 1;
            let total = self.search_results.len();
            let query = &self.search_query;
            self.status_message = format!("[{n}/{total}] /{query}");
        }
    }

    fn next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.search_index = (self.search_index + 1) % self.search_results.len();
        self.cursor = self.search_results[self.search_index];
        let n = self.search_index + 1;
        let total = self.search_results.len();
        let query = &self.search_query;
        self.status_message = format!("[{n}/{total}] /{query}");
    }

    fn prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.search_index = if self.search_index == 0 {
            self.search_results.len() - 1
        } else {
            self.search_index - 1
        };
        self.cursor = self.search_results[self.search_index];
        let n = self.search_index + 1;
        let total = self.search_results.len();
        let query = &self.search_query;
        self.status_message = format!("[{n}/{total}] /{query}");
    }

    pub const fn adjust_scroll(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.cursor - viewport_height + 1;
        } else {
            // Cursor is within visible range, no scroll adjustment needed
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.cursor = self.cursor.saturating_sub(3);
            }
            MouseEventKind::ScrollDown => {
                self.cursor = (self.cursor + 3).min(self.visible.len().saturating_sub(1));
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if mouse.row >= 1 {
                    let line_in_viewport = (mouse.row - 1) as usize;
                    let target = self.scroll_offset + line_in_viewport;
                    if target < self.visible.len() {
                        if self.cursor == target {
                            self.toggle_or_expand();
                        } else {
                            self.cursor = target;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn copy_value(&mut self) {
        if let Some(&node_idx) = self.visible.get(self.cursor) {
            let value = self.tree.node_value_string(node_idx);
            self.set_clipboard(&value, "Copied");
        }
    }

    fn copy_path(&mut self) {
        let path = self.current_path();
        self.set_clipboard(&path, "Copied path");
    }

    fn set_clipboard(&mut self, text: &str, prefix: &str) {
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text.to_owned())) {
            Ok(()) => {
                let preview = if text.chars().count() > 50 {
                    let truncated: String = text.chars().take(47).collect();
                    format!("{truncated}…")
                } else {
                    text.to_string()
                };
                self.status_message = format!("{prefix}: {preview}");
            }
            Err(_) => {
                self.status_message = "Failed to copy to clipboard".to_string();
            }
        }
    }

    fn expand_to_depth(&mut self, depth: usize) {
        self.tree.expand_to_depth(depth);
        self.refresh_visible();
        self.status_message = format!("Expanded to depth {depth}");
    }
}
