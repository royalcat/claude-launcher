use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

// Brand accent: warm orange (ANSI 256 color 208)
pub const ORANGE: Color = Color::Indexed(208);
pub const DIM_COLOR: Color = Color::Gray;
pub const GREEN: Color = Color::Green;
pub const RED: Color = Color::Red;
pub const YELLOW: Color = Color::Yellow;
pub const CYAN: Color = Color::Cyan;

pub fn orange(text: &str) -> Span<'static> {
    Span::styled(text.to_string(), Style::default().fg(ORANGE).add_modifier(Modifier::BOLD))
}

pub fn dim(text: &str) -> Span<'static> {
    Span::styled(text.to_string(), Style::default().fg(DIM_COLOR))
}

pub fn selected_item() -> Style {
    Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
}
