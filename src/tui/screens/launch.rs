use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::widgets::{SelectList, render_footer, render_status};
use crate::config::{get_all_credentials, mask_secret};
use crate::tui::theme::*;

pub struct LaunchState {
    list: SelectList,
    /// Ordered slugs matching list items
    slugs: Vec<String>,
    status: String,
    is_error: bool,
    empty: bool,
}

impl LaunchState {
    pub fn new() -> Self {
        let all = get_all_credentials().unwrap_or_default();
        let mut slugs: Vec<String> = all.keys().cloned().collect();
        slugs.sort();

        let empty = slugs.is_empty();

        let items: Vec<(String, String)> = if empty {
            vec![("Add credentials".to_string(), "".to_string())]
        } else {
            slugs
                .iter()
                .map(|s| {
                    let c = &all[s];
                    let token = c
                        .env
                        .get("ANTHROPIC_AUTH_TOKEN")
                        .map(|t| mask_secret(t))
                        .unwrap_or_else(|| "no key".to_string());
                    (c.name.clone(), format!("{} · {}", c.provider, token))
                })
                .collect()
        };

        LaunchState {
            list: SelectList::new(items),
            slugs,
            status: String::new(),
            is_error: false,
            empty,
        }
    }
}

pub enum Nav {
    None,
    Back,
    Launch { slug: String, claude_args: Vec<String> },
    AddCredentials,
}

pub fn render(f: &mut Frame, state: &mut LaunchState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // title
            Constraint::Min(8),    // list
            Constraint::Length(2), // status
            Constraint::Length(2), // footer
        ])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("Select credentials to launch with:", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        dim("(/ to filter)"),
    ]));
    f.render_widget(title, chunks[0]);

    state.list.render(f, chunks[1], true);
    render_status(f, chunks[2], &state.status, state.is_error);
    render_footer(f, chunks[3], &[("↑↓", "move"), ("Enter", "launch"), ("Esc", "back"), ("/", "filter")]);
}

pub fn handle_key(state: &mut LaunchState, key: KeyEvent) -> Nav {
    // Filter input mode: any printable key updates filter
    if key.code == KeyCode::Esc {
        if !state.list.filter.is_empty() {
            state.list.filter.clear();
            state.list.filter_items();
            return Nav::None;
        }
        return Nav::Back;
    }

    match key.code {
        KeyCode::Char('/') if state.list.filter.is_empty() => {
            // Enter filter mode: handled by subsequent keypresses
            Nav::None
        }
        KeyCode::Char(c) if !state.list.filter.is_empty() || c == '/' => {
            if c != '/' {
                state.list.filter.push(c);
                state.list.filter_items();
            }
            Nav::None
        }
        KeyCode::Backspace => {
            if !state.list.filter.is_empty() {
                state.list.filter.pop();
                state.list.filter_items();
            }
            Nav::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.list.move_up();
            Nav::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.list.move_down();
            Nav::None
        }
        KeyCode::Enter => {
            if state.empty {
                return Nav::AddCredentials;
            }
            if let Some(idx) = state.list.selected_original_index() {
                let slug = state.slugs[idx].clone();
                Nav::Launch { slug, claude_args: vec![] }
            } else {
                Nav::None
            }
        }
        _ => Nav::None,
    }
}
