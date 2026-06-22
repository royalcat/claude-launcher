use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::theme::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BANNER_HEIGHT: u16 = 3;

/// Render the banner bar at the top of the screen.
pub fn render_banner(f: &mut Frame, area: Rect, workspace_label: &str) {
    let cwd = std::env::current_dir().unwrap_or_default().to_string_lossy().into_owned();
    let home = dirs::home_dir().unwrap_or_default().to_string_lossy().into_owned();
    let short_cwd = if cwd.starts_with(&home) {
        format!("~{}", &cwd[home.len()..])
    } else {
        cwd
    };

    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(DIM_COLOR));

    let inner = block.inner(area);
    f.render_widget(&block, area);

    let chunks = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).split(inner);

    let title_line = Line::from(vec![
        Span::raw(" "),
        orange("claude-launcher"),
        dim(&format!("  ·  v{VERSION}  ·  {short_cwd}")),
    ]);
    let workspace_line = Line::from(vec![
        dim("Workspace: "),
        Span::styled(workspace_label, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" "),
    ]);

    f.render_widget(Paragraph::new(title_line), chunks[0]);
    f.render_widget(Paragraph::new(workspace_line).alignment(Alignment::Right), chunks[1]);
}

/// A simple list widget with keyboard navigation.
pub struct SelectList {
    pub items: Vec<(String, String)>, // (label, description)
    pub selected: usize,
    pub filter: String,
    pub filtered_indices: Vec<usize>,
    pub scroll_offset: usize,
}

impl SelectList {
    pub fn new(items: Vec<(String, String)>) -> Self {
        let len = items.len();
        let s = SelectList {
            items,
            selected: 0,
            filter: String::new(),
            filtered_indices: (0..len).collect(),
            scroll_offset: 0,
        };
        s
    }

    pub fn filter_items(&mut self) {
        let q = self.filter.to_lowercase();
        self.filtered_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, (label, desc))| q.is_empty() || label.to_lowercase().contains(&q) || desc.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        self.selected = self.selected.min(self.filtered_indices.len().saturating_sub(1));
        self.scroll_offset = 0;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered_indices.len() {
            self.selected += 1;
        }
    }

    pub fn selected_original_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected).copied()
    }

    pub fn render(&self, f: &mut Frame, area: Rect, show_filter: bool) {
        let visible_height = area.height as usize;

        // Compute scroll window
        let scroll_start = self.scroll_offset;
        let scroll_end = (scroll_start + visible_height).min(self.filtered_indices.len());

        let mut lines: Vec<Line> = Vec::new();

        if show_filter && !self.filter.is_empty() {
            lines.push(Line::from(vec![dim("  Filter: "), Span::raw(self.filter.clone())]));
        }

        for (visible_i, filtered_i) in self.filtered_indices[scroll_start..scroll_end].iter().enumerate() {
            let abs_i = scroll_start + visible_i;
            let (label, desc) = &self.items[*filtered_i];
            let is_selected = abs_i == self.selected;

            let cursor = if is_selected {
                Span::styled("❯ ", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("  ")
            };

            let label_span = if is_selected {
                Span::styled(label.clone(), selected_item())
            } else {
                Span::styled(label.clone(), Style::default().add_modifier(Modifier::BOLD))
            };

            let line = if desc.is_empty() {
                Line::from(vec![cursor, label_span])
            } else {
                let pad = " ".repeat(2);
                Line::from(vec![cursor, label_span, Span::raw(pad), dim(&format!("— {desc}"))])
            };
            lines.push(line);
        }

        let p = Paragraph::new(lines);
        f.render_widget(p, area);
    }
}

/// State for a choice-picker popup overlaid on a form field.
pub struct ChoicePicker {
    pub field_index: usize,
    pub list: SelectList,
    pub options: Vec<String>,
}

/// Create a centered rectangle with given percentages of the available area.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let x_offset = (100 - percent_x) / 2;
    let y_offset = (100 - percent_y) / 2;
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(y_offset),
            Constraint::Percentage(percent_y),
            Constraint::Percentage(y_offset),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(x_offset),
            Constraint::Percentage(percent_x),
            Constraint::Percentage(x_offset),
        ])
        .split(popup_layout[1])[1]
}

/// Render a footer hint line.
pub fn render_footer(f: &mut Frame, area: Rect, hints: &[(&str, &str)]) {
    let spans: Vec<Span> = hints
        .iter()
        .enumerate()
        .flat_map(|(i, (key, action))| {
            let mut parts = vec![
                Span::styled(key.to_string(), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {action}")),
            ];
            if i + 1 < hints.len() {
                parts.push(dim("  ·  "));
            }
            parts
        })
        .collect();

    let line = Line::from(spans);
    let p = Paragraph::new(vec![Line::raw(""), line]).alignment(Alignment::Left);
    f.render_widget(p, area);
}

/// A simple status message (error/success) shown below the main content.
pub fn render_status(f: &mut Frame, area: Rect, msg: &str, is_error: bool) {
    if msg.is_empty() {
        return;
    }
    let color = if is_error { RED } else { GREEN };
    let p = Paragraph::new(Line::from(vec![Span::raw("  "), Span::styled(msg, Style::default().fg(color))]));
    f.render_widget(p, area);
}
