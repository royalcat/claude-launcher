use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_textarea::TextArea;

use super::widgets::{SelectList, render_footer, render_status};
use crate::settings::{
    add_profile, default_config_path, get_active_profile, list_profiles, remove_profile, rename_profile, set_active_profile, slugify_label,
    update_profile_path,
};
use crate::tui::theme::*;

#[derive(Debug, Clone, PartialEq)]
enum Step {
    TopMenu,
    SwitchProfile,
    AddProfile { sub: AddProfileSub },
    EditProfileMenu,
    EditProfileAction { target: String, sub: EditSub },
    DeleteProfile,
}

#[derive(Debug, Clone, PartialEq)]
enum AddProfileSub {
    Label,
    Path,
}

#[derive(Debug, Clone, PartialEq)]
enum EditSub {
    ActionMenu,
    EditPath,
    Rename,
}

const TOP_ITEMS: &[(&str, &str, &str)] = &[
    ("Switch profile", "pick which profile is active", "switch"),
    ("Add profile", "register a new credentials file", "add"),
    ("Edit profile", "rename or change a profile path", "edit"),
    ("Delete profile", "remove a profile (cannot be active)", "delete"),
    ("Back", "", "back"),
];

pub struct SettingsState {
    step: Step,
    menu_list: SelectList,
    profile_list: SelectList,
    profile_slugs: Vec<String>,
    text_input: TextArea<'static>,
    text_input2: TextArea<'static>,
    status: String,
    is_error: bool,
}

impl SettingsState {
    pub fn new() -> Self {
        let menu_items = TOP_ITEMS.iter().map(|(l, d, _)| (l.to_string(), d.to_string())).collect();
        SettingsState {
            step: Step::TopMenu,
            menu_list: SelectList::new(menu_items),
            profile_list: SelectList::new(vec![]),
            profile_slugs: vec![],
            text_input: TextArea::default(),
            text_input2: TextArea::default(),
            status: String::new(),
            is_error: false,
        }
    }

    fn refresh_profile_list(&mut self) {
        let profiles = list_profiles();
        let (active, _) = get_active_profile();
        let mut slugs: Vec<String> = profiles.keys().cloned().collect();
        slugs.sort();
        let items: Vec<(String, String)> = slugs
            .iter()
            .map(|s| {
                let label = if s == &active { format!("{} (active)", s) } else { s.clone() };
                (label, profiles[s].clone())
            })
            .collect();
        self.profile_slugs = slugs;
        self.profile_list = SelectList::new(items);
    }
}

pub enum Nav {
    None,
    Back,
}

pub fn render(f: &mut Frame, state: &mut SettingsState) {
    let area = f.area();
    let (active_label, active_path) = get_active_profile();

    match &state.step.clone() {
        Step::TopMenu => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(4), Constraint::Min(8), Constraint::Length(2), Constraint::Length(2)])
                .split(area);

            let title = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Settings", Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    dim("Active profile: "),
                    Span::styled(&active_label, Style::default().add_modifier(Modifier::BOLD)),
                    dim(&format!("  ·  {active_path}")),
                ]),
            ]);
            f.render_widget(title, chunks[0]);
            state.menu_list.render(f, chunks[1], false);
            render_status(f, chunks[2], &state.status, state.is_error);
            render_footer(f, chunks[3], &[("↑↓", "move"), ("Enter", "select"), ("Esc", "back")]);
        }

        Step::SwitchProfile => {
            render_profile_picker(f, state, area, "Switch active profile to:");
        }

        Step::AddProfile { sub } => {
            let (prompt, hint) = match sub {
                AddProfileSub::Label => ("Profile label:", "e.g. personal, office, cheap-models"),
                AddProfileSub::Path => ("Path to credentials JSON:", "~ expands to home directory"),
            };
            render_text_input_step(f, state, area, "Add Profile", prompt, hint);
        }

        Step::EditProfileMenu => {
            render_profile_picker(f, state, area, "Edit which profile?");
        }

        Step::EditProfileAction { target, sub } => {
            let target = target.clone();
            let sub = sub.clone();
            match sub {
                EditSub::ActionMenu => {
                    render_edit_action_menu(f, state, area, &target);
                }
                EditSub::EditPath => {
                    render_text_input_step(
                        f,
                        state,
                        area,
                        &format!("Edit Path — {target}"),
                        "New path:",
                        "~ expands to home directory",
                    );
                }
                EditSub::Rename => {
                    render_text_input_step(f, state, area, &format!("Rename — {target}"), "New label:", "");
                }
            }
        }

        Step::DeleteProfile => {
            render_profile_picker(f, state, area, "Delete which profile?");
        }
    }
}

fn render_profile_picker(f: &mut Frame, state: &mut SettingsState, area: Rect, title_str: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(8), Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(title_str, Style::default().add_modifier(Modifier::BOLD)),
    ]));
    f.render_widget(title, chunks[0]);
    state.profile_list.render(f, chunks[1], false);
    render_status(f, chunks[2], &state.status, state.is_error);
    render_footer(f, chunks[3], &[("↑↓", "move"), ("Enter", "select"), ("Esc", "back")]);
}

fn render_text_input_step(f: &mut Frame, state: &mut SettingsState, area: Rect, screen_title: &str, prompt: &str, hint: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(screen_title, Style::default().add_modifier(Modifier::BOLD)),
    ]));
    f.render_widget(title, chunks[0]);

    if !hint.is_empty() {
        let hint_p = Paragraph::new(Line::from(vec![Span::raw("  "), dim(hint)]));
        f.render_widget(hint_p, chunks[1]);
    }

    let block = Block::default()
        .title(format!("  {prompt} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE));
    state.text_input.set_block(block);
    state.text_input.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_widget(&state.text_input, chunks[2]);

    render_status(f, chunks[4], &state.status, state.is_error);
    render_footer(f, chunks[5], &[("Enter", "confirm"), ("Esc", "cancel")]);
}

fn render_edit_action_menu(f: &mut Frame, state: &mut SettingsState, area: Rect, target: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(6), Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("Edit \"{target}\":"), Style::default().add_modifier(Modifier::BOLD)),
    ]));
    f.render_widget(title, chunks[0]);
    state.menu_list.render(f, chunks[1], false);
    render_status(f, chunks[2], &state.status, state.is_error);
    render_footer(f, chunks[3], &[("↑↓", "move"), ("Enter", "select"), ("Esc", "back")]);
}

pub fn handle_key(state: &mut SettingsState, key: KeyEvent) -> Nav {
    match state.step.clone() {
        Step::TopMenu => handle_top_menu(state, key),
        Step::SwitchProfile => handle_switch(state, key),
        Step::AddProfile { ref sub } => handle_add_profile(state, key, sub.clone()),
        Step::EditProfileMenu => handle_edit_menu(state, key),
        Step::EditProfileAction { ref target, ref sub } => handle_edit_action(state, key, target.clone(), sub.clone()),
        Step::DeleteProfile => handle_delete_profile(state, key),
    }
}

fn handle_top_menu(state: &mut SettingsState, key: KeyEvent) -> Nav {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Nav::Back,
        KeyCode::Up | KeyCode::Char('k') => {
            state.menu_list.move_up();
            Nav::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.menu_list.move_down();
            Nav::None
        }
        KeyCode::Enter => {
            if let Some(idx) = state.menu_list.selected_original_index() {
                let action = TOP_ITEMS[idx].2;
                state.status.clear();
                match action {
                    "back" => return Nav::Back,
                    "switch" => {
                        state.refresh_profile_list();
                        state.step = Step::SwitchProfile;
                    }
                    "add" => {
                        state.text_input = TextArea::default();
                        state.text_input2 = TextArea::default();
                        // Insert default path
                        state.text_input2.insert_str(&default_config_path().to_string_lossy());
                        state.step = Step::AddProfile { sub: AddProfileSub::Label };
                    }
                    "edit" => {
                        state.refresh_profile_list();
                        state.step = Step::EditProfileMenu;
                    }
                    "delete" => {
                        state.refresh_profile_list();
                        state.step = Step::DeleteProfile;
                    }
                    _ => {}
                }
            }
            Nav::None
        }
        _ => Nav::None,
    }
}

fn handle_switch(state: &mut SettingsState, key: KeyEvent) -> Nav {
    match key.code {
        KeyCode::Esc => {
            state.step = Step::TopMenu;
            Nav::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.profile_list.move_up();
            Nav::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.profile_list.move_down();
            Nav::None
        }
        KeyCode::Enter => {
            if let Some(idx) = state.profile_list.selected_original_index() {
                let slug = state.profile_slugs[idx].clone();
                match set_active_profile(&slug) {
                    Ok(_) => {
                        state.status = format!("Active profile is now \"{slug}\".");
                        state.is_error = false;
                        state.step = Step::TopMenu;
                    }
                    Err(e) => {
                        state.status = e.to_string();
                        state.is_error = true;
                    }
                }
            }
            Nav::None
        }
        _ => Nav::None,
    }
}

fn handle_add_profile(state: &mut SettingsState, key: KeyEvent, sub: AddProfileSub) -> Nav {
    match key.code {
        KeyCode::Esc => {
            state.step = Step::TopMenu;
            Nav::None
        }
        KeyCode::Enter => {
            match sub {
                AddProfileSub::Label => {
                    let label = state.text_input.lines().join("").trim().to_string();
                    let slug = slugify_label(&label);
                    if slug.is_empty() {
                        state.status = "Label needs at least one alphanumeric character".to_string();
                        state.is_error = true;
                        return Nav::None;
                    }
                    if list_profiles().contains_key(&slug) {
                        state.status = format!("Profile \"{slug}\" already exists.");
                        state.is_error = true;
                        return Nav::None;
                    }
                    // Move to path step
                    state.status.clear();
                    state.step = Step::AddProfile { sub: AddProfileSub::Path };
                    Nav::None
                }
                AddProfileSub::Path => {
                    let label_raw = state.text_input.lines().join("").trim().to_string();
                    let slug = slugify_label(&label_raw);
                    let path_raw = state.text_input2.lines().join("").trim().to_string();
                    let path = crate::settings::expand_path(&path_raw);
                    if path.is_empty() {
                        state.status = "Path is required".to_string();
                        state.is_error = true;
                        return Nav::None;
                    }
                    match add_profile(&slug, &path) {
                        Ok(final_slug) => {
                            state.status = format!("Profile \"{final_slug}\" added.");
                            state.is_error = false;
                            state.step = Step::TopMenu;
                        }
                        Err(e) => {
                            state.status = e.to_string();
                            state.is_error = true;
                        }
                    }
                    Nav::None
                }
            }
        }
        _ => {
            match sub {
                AddProfileSub::Label => {
                    state.text_input.input(key);
                }
                AddProfileSub::Path => {
                    state.text_input2.input(key);
                }
            }
            Nav::None
        }
    }
}

fn handle_edit_menu(state: &mut SettingsState, key: KeyEvent) -> Nav {
    match key.code {
        KeyCode::Esc => {
            state.step = Step::TopMenu;
            Nav::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.profile_list.move_up();
            Nav::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.profile_list.move_down();
            Nav::None
        }
        KeyCode::Enter => {
            if let Some(idx) = state.profile_list.selected_original_index() {
                let slug = state.profile_slugs[idx].clone();
                // Show action menu
                let action_items = vec![
                    ("Change path".to_string(), "point this profile at a different file".to_string()),
                    ("Rename".to_string(), "change the label".to_string()),
                    ("Back".to_string(), "".to_string()),
                ];
                state.menu_list = SelectList::new(action_items);
                state.step = Step::EditProfileAction {
                    target: slug,
                    sub: EditSub::ActionMenu,
                };
            }
            Nav::None
        }
        _ => Nav::None,
    }
}

fn handle_edit_action(state: &mut SettingsState, key: KeyEvent, target: String, sub: EditSub) -> Nav {
    match sub {
        EditSub::ActionMenu => {
            match key.code {
                KeyCode::Esc => {
                    state.step = Step::TopMenu;
                    Nav::None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.menu_list.move_up();
                    Nav::None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    state.menu_list.move_down();
                    Nav::None
                }
                KeyCode::Enter => {
                    let idx = state.menu_list.selected_original_index().unwrap_or(2);
                    match idx {
                        0 => {
                            // Change path
                            let profiles = list_profiles();
                            let old_path = profiles.get(&target).cloned().unwrap_or_default();
                            state.text_input = TextArea::default();
                            state.text_input.insert_str(&old_path);
                            state.step = Step::EditProfileAction {
                                target,
                                sub: EditSub::EditPath,
                            };
                        }
                        1 => {
                            // Rename
                            state.text_input = TextArea::default();
                            state.text_input.insert_str(&target);
                            state.step = Step::EditProfileAction {
                                target,
                                sub: EditSub::Rename,
                            };
                        }
                        _ => {
                            state.step = Step::TopMenu;
                        }
                    }
                    Nav::None
                }
                _ => Nav::None,
            }
        }
        EditSub::EditPath => match key.code {
            KeyCode::Esc => {
                state.step = Step::TopMenu;
                Nav::None
            }
            KeyCode::Enter => {
                let new_path_raw = state.text_input.lines().join("").trim().to_string();
                let new_path = crate::settings::expand_path(&new_path_raw);
                match update_profile_path(&target, &new_path) {
                    Ok(_) => {
                        state.status = format!("Path updated for \"{target}\".");
                        state.is_error = false;
                        state.step = Step::TopMenu;
                    }
                    Err(e) => {
                        state.status = e.to_string();
                        state.is_error = true;
                    }
                }
                Nav::None
            }
            _ => {
                state.text_input.input(key);
                Nav::None
            }
        },
        EditSub::Rename => match key.code {
            KeyCode::Esc => {
                state.step = Step::TopMenu;
                Nav::None
            }
            KeyCode::Enter => {
                let new_label = state.text_input.lines().join("").trim().to_string();
                match rename_profile(&target, &new_label) {
                    Ok(new_slug) => {
                        state.status = format!("Renamed \"{target}\" → \"{new_slug}\".");
                        state.is_error = false;
                        state.step = Step::TopMenu;
                    }
                    Err(e) => {
                        state.status = e.to_string();
                        state.is_error = true;
                    }
                }
                Nav::None
            }
            _ => {
                state.text_input.input(key);
                Nav::None
            }
        },
    }
}

fn handle_delete_profile(state: &mut SettingsState, key: KeyEvent) -> Nav {
    match key.code {
        KeyCode::Esc => {
            state.step = Step::TopMenu;
            Nav::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.profile_list.move_up();
            Nav::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.profile_list.move_down();
            Nav::None
        }
        KeyCode::Enter => {
            if let Some(idx) = state.profile_list.selected_original_index() {
                let slug = state.profile_slugs[idx].clone();
                match remove_profile(&slug) {
                    Ok(_) => {
                        state.status = format!("Profile \"{slug}\" deleted.");
                        state.is_error = false;
                        state.step = Step::TopMenu;
                    }
                    Err(e) => {
                        state.status = e.to_string();
                        state.is_error = true;
                    }
                }
            }
            Nav::None
        }
        _ => Nav::None,
    }
}
