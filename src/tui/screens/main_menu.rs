use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use super::Screen;
use super::widgets::{SelectList, render_banner, render_footer};

const MENU_ITEMS: &[(&str, &str, &str)] = &[
    ("Launch Last", "", "last"),
    ("Launch with provider", "run Claude Code with saved credentials", "use"),
    ("Launch default", "run with official Anthropic settings", "default"),
    ("Add credentials", "save new credentials for a provider", "add"),
    ("Edit credentials", "modify a saved set", "edit"),
    ("Delete credentials", "remove a saved set", "delete"),
    ("Settings", "configure claude-launcher", "settings"),
    ("Help", "about claude-launcher", "help"),
    ("Exit", "", "exit"),
];

pub struct MainMenuState {
    list: SelectList,
    actions: Vec<&'static str>,
}

impl MainMenuState {
    pub fn new(last_launched_credential: Option<String>) -> Self {
        let mut actions: Vec<&'static str> = Vec::new();
        let items: Vec<(String, String)> = MENU_ITEMS
            .iter()
            .filter_map(|(label, desc, slug)| {
                if *slug == "last" {
                    last_launched_credential.as_ref().map(|cred_slug| {
                        actions.push(*slug);
                        let name = crate::config::get_all_credentials()
                            .ok()
                            .and_then(|creds| creds.get(cred_slug).map(|c| c.name.clone()))
                            .unwrap_or_else(|| cred_slug.clone());
                        (label.to_string(), name)
                    })
                } else {
                    actions.push(*slug);
                    Some((label.to_string(), desc.to_string()))
                }
            })
            .collect();
        MainMenuState {
            list: SelectList::new(items),
            actions,
        }
    }
}

pub enum Nav {
    None,
    Exit,
    LaunchDefault,
    LaunchLast,
    GoTo(Screen),
}

pub fn render(f: &mut Frame, state: &mut MainMenuState, profile_label: &str, profile_path: &str, _last_launched_credential: &Option<String>) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // banner
            Constraint::Min(10),   // menu list
            Constraint::Length(2), // footer
        ])
        .split(area);

    render_banner(f, chunks[0], profile_label, profile_path);
    state.list.render(f, chunks[1], false);
    render_footer(f, chunks[2], &[("↑↓", "move"), ("Enter", "select"), ("q", "quit")]);
}

pub fn handle_key(state: &mut MainMenuState, key: KeyEvent) -> Nav {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            state.list.move_up();
            Nav::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.list.move_down();
            Nav::None
        }
        KeyCode::Enter => {
            if let Some(idx) = state.list.selected_original_index() {
                let action = state.actions[idx];
                match action {
                    "exit" => Nav::Exit,
                    "default" => Nav::LaunchDefault,
                    "last" => Nav::LaunchLast,
                    "use" => Nav::GoTo(Screen::Launch(super::launch::LaunchState::new())),
                    "add" => Nav::GoTo(Screen::Add(super::add::AddState::new())),
                    "edit" => Nav::GoTo(Screen::Edit(super::edit::EditState::new())),
                    "delete" => Nav::GoTo(Screen::Delete(super::delete::DeleteState::new())),
                    "settings" => Nav::GoTo(Screen::Settings(super::settings::SettingsState::new())),
                    "help" => Nav::GoTo(Screen::Help),
                    _ => Nav::None,
                }
            } else {
                Nav::None
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => Nav::Exit,
        _ => Nav::None,
    }
}
