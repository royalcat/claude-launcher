use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::widgets::{SelectList, render_footer, render_status};
use crate::config::{get_all_credentials, remove_credential};
use crate::tui::theme::*;

enum Step {
    PickCredential,
    Confirm,
}

pub struct DeleteState {
    step: Step,
    list: SelectList,
    slugs: Vec<String>,
    selected_slug: Option<String>,
    selected_name: Option<String>,
    confirm_yes: bool,
    status: String,
    is_error: bool,
    empty: bool,
}

impl DeleteState {
    pub fn new() -> Self {
        let all = get_all_credentials().unwrap_or_default();
        let mut slugs: Vec<String> = all.keys().cloned().collect();
        slugs.sort();
        let empty = slugs.is_empty();
        let items: Vec<(String, String)> = if empty {
            vec![("Add credentials".to_string(), "".to_string())]
        } else {
            slugs.iter().map(|s| (all[s].name.clone(), all[s].provider.clone())).collect()
        };
        DeleteState {
            step: Step::PickCredential,
            list: SelectList::new(items),
            slugs,
            selected_slug: None,
            selected_name: None,
            confirm_yes: false,
            status: String::new(),
            is_error: false,
            empty,
        }
    }
}

pub enum Nav {
    None,
    Back,
    AddCredentials,
}

pub fn render(f: &mut Frame, state: &mut DeleteState) {
    let area = f.area();
    match state.step {
        Step::PickCredential => render_picker(f, state, area),
        Step::Confirm => render_confirm(f, state, area),
    }
}

fn render_picker(f: &mut Frame, state: &mut DeleteState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(8), Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("Which credentials to delete?", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    f.render_widget(title, chunks[0]);
    state.list.render(f, chunks[1], false);
    render_status(f, chunks[2], &state.status, state.is_error);
    render_footer(f, chunks[3], &[("↑↓", "move"), ("Enter", "select"), ("Esc", "back")]);
}

fn render_confirm(f: &mut Frame, state: &mut DeleteState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let name = state.selected_name.as_deref().unwrap_or("?");
    let msg = Paragraph::new(vec![
        Line::raw(""),
        Line::from(vec![
            Span::raw("  Delete \""),
            Span::styled(name, Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)),
            Span::raw("\"? This cannot be undone."),
        ]),
    ]);
    f.render_widget(msg, chunks[0]);

    let yes_style = if state.confirm_yes {
        Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM_COLOR)
    };
    let no_style = if !state.confirm_yes {
        Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM_COLOR)
    };

    let confirm = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("[ Yes ]", yes_style),
        Span::raw("   "),
        Span::styled("[ No ]", no_style),
    ]));
    f.render_widget(confirm, chunks[1]);

    render_status(f, chunks[2], &state.status, state.is_error);
    render_footer(f, chunks[3], &[("←→", "choose"), ("Enter", "confirm"), ("Esc", "cancel")]);
}

pub fn handle_key(state: &mut DeleteState, key: KeyEvent) -> Nav {
    match state.step {
        Step::PickCredential => handle_picker_key(state, key),
        Step::Confirm => handle_confirm_key(state, key),
    }
}

fn handle_picker_key(state: &mut DeleteState, key: KeyEvent) -> Nav {
    match key.code {
        KeyCode::Esc => Nav::Back,
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
                let all = get_all_credentials().unwrap_or_default();
                let name = all.get(&slug).map(|c| c.name.clone()).unwrap_or_default();
                state.selected_slug = Some(slug);
                state.selected_name = Some(name);
                state.confirm_yes = false;
                state.step = Step::Confirm;
            }
            Nav::None
        }
        _ => Nav::None,
    }
}

fn handle_confirm_key(state: &mut DeleteState, key: KeyEvent) -> Nav {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => Nav::Back,
        KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
            state.confirm_yes = !state.confirm_yes;
            Nav::None
        }
        KeyCode::Enter => {
            if !state.confirm_yes {
                return Nav::Back;
            }
            if let Some(ref slug) = state.selected_slug.clone() {
                match remove_credential(slug) {
                    Ok(_) => {
                        state.status = format!("Credentials \"{}\" deleted.", state.selected_name.as_deref().unwrap_or(slug));
                        state.is_error = false;
                        Nav::Back
                    }
                    Err(e) => {
                        state.status = format!("Failed to delete: {e}");
                        state.is_error = true;
                        Nav::None
                    }
                }
            } else {
                Nav::Back
            }
        }
        _ => Nav::None,
    }
}
