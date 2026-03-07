use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crate::app::{App, InputMode};
use crate::tree::NodeKind;

const CLR_KEY: Color = Color::Cyan;
const CLR_STRING: Color = Color::Green;
const CLR_NUMBER: Color = Color::Yellow;
const CLR_BOOL: Color = Color::Magenta;
const CLR_NULL: Color = Color::Red;
const CLR_BRACKET: Color = Color::White;
const CLR_DIM: Color = Color::DarkGray;
const CLR_SELECTED_BG: Color = Color::Rgb(30, 30, 50);
const CLR_SEARCH_BG: Color = Color::Rgb(60, 50, 20);
const CLR_BAR_BG: Color = Color::Rgb(24, 24, 36);
const CLR_BAR_FG: Color = Color::Rgb(140, 140, 180);

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_tree(f, app, chunks[0]);
    draw_path_bar(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);

    if app.show_help {
        draw_help_popup(f);
    }
}

fn draw_tree(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLR_DIM))
        .title(Span::styled(
            format!(" {} ", app.filename),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let viewport_height = inner.height as usize;
    app.adjust_scroll(viewport_height);

    let lines: Vec<Line> = app
        .visible
        .iter()
        .enumerate()
        .skip(app.scroll_offset)
        .take(viewport_height)
        .map(|(i, &node_idx)| render_node(app, i, node_idx))
        .collect();

    f.render_widget(Paragraph::new(lines), inner);

    if app.visible.len() > viewport_height {
        let mut scrollbar_state =
            ScrollbarState::new(app.visible.len().saturating_sub(viewport_height))
                .position(app.scroll_offset);
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

fn render_node<'a>(app: &'a App, line_index: usize, node_idx: usize) -> Line<'a> {
    let node = &app.tree.nodes[node_idx];
    let is_selected = line_index == app.cursor;
    let is_search_hit = app.search_results.contains(&line_index);

    let indent = "  ".repeat(node.depth);
    let mut spans: Vec<Span> = Vec::new();

    spans.push(Span::raw(indent));

    if node.is_expandable() {
        let arrow = if node.expanded { "▼ " } else { "▶ " };
        spans.push(Span::styled(arrow, Style::default().fg(CLR_DIM)));
    } else {
        spans.push(Span::raw("  "));
    }

    if let Some(key) = &node.key {
        if node.is_array_element {
            spans.push(Span::styled(
                format!("[{}]", key),
                Style::default().fg(CLR_DIM),
            ));
        } else {
            spans.push(Span::styled(
                format!("\"{}\"", key),
                Style::default().fg(CLR_KEY).add_modifier(Modifier::BOLD),
            ));
        }
        spans.push(Span::styled(": ", Style::default().fg(CLR_DIM)));
    }

    match &node.kind {
        NodeKind::Object(n) => {
            if *n == 0 {
                spans.push(Span::styled("{}", Style::default().fg(CLR_BRACKET)));
            } else {
                let summary = format!(
                    "{{{}}}",
                    if node.expanded {
                        format!("{} field{}", n, if *n == 1 { "" } else { "s" })
                    } else {
                        format!("…{}", n)
                    }
                );
                spans.push(Span::styled(summary, Style::default().fg(CLR_DIM)));
            }
        }
        NodeKind::Array(n) => {
            if *n == 0 {
                spans.push(Span::styled("[]", Style::default().fg(CLR_BRACKET)));
            } else {
                let summary = format!(
                    "[{}]",
                    if node.expanded {
                        format!("{} item{}", n, if *n == 1 { "" } else { "s" })
                    } else {
                        format!("…{}", n)
                    }
                );
                spans.push(Span::styled(summary, Style::default().fg(CLR_DIM)));
            }
        }
        NodeKind::String(s) => {
            let display = if s.len() > 120 {
                format!("\"{}…\"", &s[..117])
            } else {
                format!("\"{}\"", s)
            };
            spans.push(Span::styled(display, Style::default().fg(CLR_STRING)));
        }
        NodeKind::Number(n) => {
            spans.push(Span::styled(n.to_string(), Style::default().fg(CLR_NUMBER)));
        }
        NodeKind::Bool(b) => {
            spans.push(Span::styled(b.to_string(), Style::default().fg(CLR_BOOL)));
        }
        NodeKind::Null => {
            spans.push(Span::styled("null", Style::default().fg(CLR_NULL)));
        }
    }

    let bg = if is_selected {
        CLR_SELECTED_BG
    } else if is_search_hit {
        CLR_SEARCH_BG
    } else {
        Color::Reset
    };

    Line::from(spans).style(Style::default().bg(bg))
}

fn draw_path_bar(f: &mut Frame, app: &App, area: Rect) {
    let path = app.current_path();
    let node_count = format!("{} nodes", app.visible.len());

    let available = area.width as usize;
    let right_len = node_count.len() + 1;
    let left_max = available.saturating_sub(right_len + 1);
    let path_display = if path.len() > left_max {
        format!("…{}", &path[path.len() - left_max + 1..])
    } else {
        path
    };

    let padding = available
        .saturating_sub(path_display.len())
        .saturating_sub(right_len);

    let line = Line::from(vec![
        Span::styled(format!(" {}", path_display), Style::default().fg(CLR_KEY)),
        Span::raw(" ".repeat(padding)),
        Span::styled(format!("{} ", node_count), Style::default().fg(CLR_DIM)),
    ]);

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(CLR_BAR_BG)),
        area,
    );
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (left, right) = if app.input_mode == InputMode::Search {
        (
            Span::styled(
                format!(" /{}", app.search_query),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                " Enter:confirm  Esc:cancel ",
                Style::default().fg(CLR_BAR_FG),
            ),
        )
    } else if !app.status_message.is_empty() {
        (
            Span::styled(
                format!(" {}", app.status_message),
                Style::default().fg(CLR_BAR_FG),
            ),
            Span::styled(" ?:help  q:quit ", Style::default().fg(CLR_DIM)),
        )
    } else {
        (
            Span::styled(
                " ↑↓/jk:move  ←→/hl:collapse/expand  e/c:all  /:search  ?:help  q:quit ",
                Style::default().fg(CLR_BAR_FG),
            ),
            Span::raw(""),
        )
    };

    let left_len = left.width();
    let right_len = right.width();
    let padding = (area.width as usize).saturating_sub(left_len + right_len);

    let line = Line::from(vec![left, Span::raw(" ".repeat(padding)), right]);

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(CLR_BAR_BG)),
        area,
    );
}

fn draw_help_popup(f: &mut Frame) {
    let area = f.area();
    let popup_width = 48u16.min(area.width.saturating_sub(4));
    let popup_height = 22u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let help_entries = [
        ("j / ↓", "Move down"),
        ("k / ↑", "Move up"),
        ("h / ←", "Collapse / go to parent"),
        ("l / → / Enter", "Expand / toggle"),
        ("g", "Go to top"),
        ("G", "Go to bottom"),
        ("f / PageDown", "Page down"),
        ("b / PageUp", "Page up"),
        ("e", "Expand all nodes"),
        ("c", "Collapse all nodes"),
        ("1-9", "Expand to depth N"),
        ("/", "Search"),
        ("n / N", "Next / previous match"),
        ("y", "Copy value to clipboard"),
        ("Y", "Copy path to clipboard"),
        ("q / Esc", "Quit"),
        ("?", "Toggle this help"),
    ];

    let lines: Vec<Line> = help_entries
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(
                    format!("  {:18}", key),
                    Style::default().fg(CLR_KEY).add_modifier(Modifier::BOLD),
                ),
                Span::styled(*desc, Style::default().fg(CLR_BAR_FG)),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLR_KEY))
        .title(Span::styled(
            " Keybindings ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::vertical(1));

    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}
