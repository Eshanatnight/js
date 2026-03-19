use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs};

use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

const CLR_DIR: Color = Color::Cyan;
const CLR_FILE: Color = Color::Green;
const CLR_DIM: Color = Color::DarkGray;
const CLR_SELECTED_BG: Color = Color::Rgb(30, 30, 50);
const CLR_BAR_BG: Color = Color::Rgb(24, 24, 36);
const CLR_BAR_FG: Color = Color::Rgb(140, 140, 180);

#[derive(Clone)]
struct Entry {
    name: String,
    is_dir: bool,
    path: PathBuf,
}

pub struct FilePicker {
    cwd: PathBuf,
    entries: Vec<Entry>,
    cursor: usize,
    scroll_offset: usize,
    show_hidden: bool,
}

impl FilePicker {
    pub fn new() -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut picker = Self {
            cwd,
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            show_hidden: false,
        };
        picker.scan_dir();
        picker
    }

    fn scan_dir(&mut self) {
        self.entries.clear();
        self.cursor = 0;
        self.scroll_offset = 0;

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        if let Ok(read) = fs::read_dir(&self.cwd) {
            for entry in read.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    dirs.push(Entry {
                        name,
                        is_dir: true,
                        path,
                    });
                } else if Path::new(&name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                {
                    files.push(Entry {
                        name,
                        is_dir: false,
                        path,
                    });
                } else {
                    // Skip non-JSON files
                }
            }
        }

        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        self.entries.push(Entry {
            name: "..".to_string(),
            is_dir: true,
            path: self.cwd.join(".."),
        });

        self.entries.extend(dirs);
        self.entries.extend(files);
    }

    fn enter_selected(&mut self) -> Option<PathBuf> {
        let entry = self.entries.get(self.cursor)?.clone();
        if entry.is_dir {
            self.navigate_to(&entry.path);
            None
        } else {
            Some(entry.path)
        }
    }

    fn navigate_to(&mut self, path: &Path) {
        if let Ok(canonical) = fs::canonicalize(path) {
            self.cwd = canonical;
            self.scan_dir();
        }
    }

    /// Run the picker and return the selected file path, or None if the user cancelled.
    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<Option<PathBuf>> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(None);
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            if self.cursor + 1 < self.entries.len() {
                                self.cursor += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            self.cursor = self.cursor.saturating_sub(1);
                        }
                        KeyCode::Char('g') => self.cursor = 0,
                        KeyCode::Char('G') => {
                            self.cursor = self.entries.len().saturating_sub(1);
                        }
                        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                            if let Some(path) = self.enter_selected() {
                                return Ok(Some(path));
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                            let parent = self.cwd.join("..");
                            self.navigate_to(&parent);
                        }
                        KeyCode::Char('.') => {
                            self.show_hidden = !self.show_hidden;
                            self.scan_dir();
                        }
                        _ => {}
                    },
                    Event::Mouse(mouse) => self.handle_mouse(mouse),
                    _ => {}
                }
            }
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(f.area());

        self.draw_listing(f, chunks[0]);
        self.draw_status(f, chunks[1]);
    }

    fn draw_listing(&mut self, f: &mut Frame, area: Rect) {
        let title = format!(" {} ", self.cwd.display());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(CLR_DIM))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let viewport_height = inner.height as usize;
        self.adjust_scroll(viewport_height);

        let lines: Vec<Line> = self
            .entries
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(viewport_height)
            .map(|(i, entry)| {
                let is_selected = i == self.cursor;
                let (icon, name_style) = if entry.is_dir {
                    (
                        "📁 ",
                        Style::default().fg(CLR_DIR).add_modifier(Modifier::BOLD),
                    )
                } else {
                    ("📄 ", Style::default().fg(CLR_FILE))
                };
                let (cursor_mark, bg) = if is_selected {
                    ("▸ ", CLR_SELECTED_BG)
                } else {
                    ("  ", Color::Reset)
                };
                let name_style = if is_selected {
                    name_style.add_modifier(Modifier::BOLD)
                } else {
                    name_style
                };
                Line::from(vec![
                    Span::styled(cursor_mark, Style::default().fg(Color::Yellow)),
                    Span::raw(icon),
                    Span::styled(&entry.name, name_style),
                    if entry.is_dir {
                        Span::styled("/", Style::default().fg(CLR_DIM))
                    } else {
                        Span::raw("")
                    },
                ])
                .style(Style::default().bg(bg))
            })
            .collect();

        f.render_widget(Paragraph::new(lines), inner);

        if self.entries.len() > viewport_height {
            let mut scrollbar_state =
                ScrollbarState::new(self.entries.len().saturating_sub(viewport_height))
                    .position(self.scroll_offset);
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("│"))
                    .thumb_symbol("█"),
                area,
                &mut scrollbar_state,
            );
        }
    }

    fn draw_status(&self, f: &mut Frame, area: Rect) {
        let hint = " ↑↓/jk:move  Enter/l:open  h/←:parent  .:hidden  q/Esc:cancel ";
        let count = format!("{} items ", self.entries.len().saturating_sub(1));

        let available = area.width as usize;
        let right_len = count.len();
        let padding = available.saturating_sub(hint.len() + right_len);

        let line = Line::from(vec![
            Span::styled(hint, Style::default().fg(CLR_BAR_FG)),
            Span::raw(" ".repeat(padding)),
            Span::styled(count, Style::default().fg(CLR_DIM)),
        ]);

        f.render_widget(
            Paragraph::new(line).style(Style::default().bg(CLR_BAR_BG)),
            area,
        );
    }

    const fn adjust_scroll(&mut self, viewport_height: usize) {
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
                self.cursor = (self.cursor + 3).min(self.entries.len().saturating_sub(1));
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if mouse.row >= 1 {
                    let line_in_viewport = (mouse.row - 1) as usize;
                    let target = self.scroll_offset + line_in_viewport;
                    if target < self.entries.len() {
                        self.cursor = target;
                    }
                }
            }
            _ => {}
        }
    }
}
