use super::extra_body::{build_extra_body, deserialize_value, get_nested_path, parse_extra_body, serialize_value, set_nested_path};
use super::widgets::{ChoicePicker, SelectList, centered_rect, render_footer, render_status};
use crate::config::{Profile, get_all_profiles, get_profile, mask_secret, rename_profile};
use crate::providers::{ExtraBodyValueType, FieldType, ProviderDef, get_provider};
use crate::tui::theme::*;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use ratatui_textarea::TextArea;

enum Step {
    PickProfile,
    FillForm,
}

pub struct EditState {
    step: Step,
    list: SelectList,
    slugs: Vec<String>,
    current_slug: Option<String>,
    fields: Vec<TextArea<'static>>,
    field_cursor: usize,
    status: String,
    is_error: bool,
    empty: bool,
    choice_picker: Option<ChoicePicker>,
}

impl EditState {
    pub fn new() -> Self {
        let all = get_all_profiles().unwrap_or_default();
        let mut slugs: Vec<String> = all.keys().cloned().collect();
        slugs.sort();
        let empty = slugs.is_empty();
        let items: Vec<(String, String)> = if empty {
            vec![("Add profile".to_string(), "".to_string())]
        } else {
            slugs.iter().map(|s| (all[s].name.clone(), all[s].provider.clone())).collect()
        };
        EditState {
            step: Step::PickProfile,
            list: SelectList::new(items),
            slugs,
            current_slug: None,
            fields: Vec::new(),
            field_cursor: 0,
            status: String::new(),
            is_error: false,
            empty,
            choice_picker: None,
        }
    }

    fn init_form(&mut self, slug: &str) {
        let profile = match get_profile(slug) {
            Ok(Some(c)) => c,
            _ => return,
        };
        let provider = match get_provider(&profile.provider) {
            Some(p) => p,
            None => return,
        };
        // Parse CLAUDE_CODE_EXTRA_BODY once; used for all ExtraBody fields below
        let extra_body_json = profile
            .env
            .get(crate::providers::ENV_EXTRA_BODY)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());

        self.fields = provider
            .fields
            .iter()
            .map(|f| {
                let mut ta = TextArea::default();
                let display = if let FieldType::ExtraBody { json_path, value_type } = &f.field_type {
                    extra_body_json
                        .as_ref()
                        .and_then(|v| get_nested_path(v, json_path))
                        .and_then(|v| deserialize_value(value_type, v))
                        .unwrap_or_default()
                } else {
                    let current = profile.env.get(f.key).cloned().unwrap_or_default();
                    if !current.is_empty() {
                        current
                    } else if let Some(default) = f.default {
                        default.to_string()
                    } else {
                        String::new()
                    }
                };
                if !display.is_empty() {
                    ta.insert_str(&display);
                } else if let Some(default) = f.default {
                    if !matches!(f.field_type, FieldType::ExtraBody { .. }) {
                        ta.insert_str(default);
                    }
                }
                ta
            })
            .collect();
        self.current_slug = Some(slug.to_string());
        self.field_cursor = 0;
    }
}

pub enum Nav {
    None,
    Back,
    AddProfile,
}

pub fn render(f: &mut Frame, state: &mut EditState) {
    let area = f.area();
    match state.step {
        Step::PickProfile => render_picker(f, state, area),
        Step::FillForm => render_form(f, state, area),
    }
}

fn render_picker(f: &mut Frame, state: &mut EditState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(8), Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("Which profile to edit?", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    f.render_widget(title, chunks[0]);
    state.list.render(f, chunks[1], false);
    render_status(f, chunks[2], &state.status, state.is_error);
    render_footer(f, chunks[3], &[("↑↓", "move"), ("Enter", "select"), ("Esc", "back")]);
}

fn render_form(f: &mut Frame, state: &mut EditState, area: Rect) {
    let slug = state.current_slug.as_deref().unwrap_or("");
    let profile = match get_profile(slug) {
        Ok(Some(c)) => c,
        _ => return,
    };
    let provider = match get_provider(&profile.provider) {
        Some(p) => p,
        None => return,
    };

    let field_count = state.fields.len();
    let mut constraints = vec![Constraint::Length(2)];
    for _ in &state.fields {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Min(1));
    constraints.push(Constraint::Length(2));
    constraints.push(Constraint::Length(2));

    let chunks = Layout::default().direction(Direction::Vertical).constraints(constraints).split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("Edit profile — {}", profile.name), Style::default().add_modifier(Modifier::BOLD)),
    ]));
    f.render_widget(title, chunks[0]);

    for (i, (field_def, textarea)) in provider.fields.iter().zip(state.fields.iter_mut()).enumerate() {
        let is_active = i == state.field_cursor;
        let req_marker = if field_def.required { " *" } else { " (opt)" };
        let is_choice = matches!(&field_def.field_type, FieldType::Choice { .. });
        let is_bool = matches!(
            &field_def.field_type,
            FieldType::ExtraBody { value_type: ExtraBodyValueType::Bool, .. }
        );
        let current_val = profile.env.get(field_def.key).cloned().unwrap_or_default();
        let hint = if is_bool {
            // Bool checkbox is self-explanatory — no hint needed
            String::new()
        } else if field_def.field_type == FieldType::Secret && !current_val.is_empty() {
            format!(" [current: {}]", mask_secret(&current_val))
        } else if current_val.is_empty() {
            " [not set]".to_string()
        } else {
            format!(" [current: {}]", current_val)
        };
        let border_style = if is_active {
            Style::default().fg(ORANGE)
        } else {
            Style::default().fg(DIM_COLOR)
        };

        if is_choice {
            // Render as a selectable value with (Space to pick) hint
            let pick_hint = if is_active { " (Space to pick)" } else { "" };
            let label = format!("  {}{}{}{}", field_def.label, req_marker, hint, pick_hint);
            let raw = textarea.lines().join("").trim().to_string();
            let display = if raw.is_empty() { "[select]" } else { &raw };
            let block = Block::default()
                .title(Span::styled(label, border_style))
                .borders(Borders::ALL)
                .border_style(border_style);
            let inner = block.inner(chunks[1 + i]);
            f.render_widget(block, chunks[1 + i]);
            let text_style = if is_active {
                if raw.is_empty() {
                    Style::default().fg(DIM_COLOR)
                } else {
                    Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
                }
            } else {
                if raw.is_empty() {
                    Style::default().fg(DIM_COLOR)
                } else {
                    Style::default()
                }
            };
            f.render_widget(
                Paragraph::new(Line::from(vec![Span::raw("  "), Span::styled(display, text_style)])),
                inner,
            );
        } else if is_bool {
            // Render as checkbox toggle instead of TextArea
            let label = format!("  {}{}{}", field_def.label, req_marker, hint);
            let raw = textarea.lines().join("").trim().to_lowercase();
            let is_checked = matches!(raw.as_str(), "true" | "yes" | "1");
            let checkbox_text = if is_checked { "[x] Yes" } else { "[ ] No" };
            let block = Block::default()
                .title(Span::styled(label, border_style))
                .borders(Borders::ALL)
                .border_style(border_style);
            let inner = block.inner(chunks[1 + i]);
            f.render_widget(block, chunks[1 + i]);
            let text_style = if is_active {
                Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DIM_COLOR)
            };
            f.render_widget(
                Paragraph::new(Line::from(vec![Span::raw("  "), Span::styled(checkbox_text, text_style)])),
                inner,
            );
        } else {
            let label = format!("  {}{}{}", field_def.label, req_marker, hint);
            let block = Block::default()
                .title(Span::styled(label, border_style))
                .borders(Borders::ALL)
                .border_style(border_style);
            textarea.set_block(block);

            if field_def.field_type == FieldType::Secret {
                textarea.set_mask_char('\u{2022}');
            }
            if is_active {
                textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
            } else {
                textarea.set_cursor_style(Style::default());
            }

            f.render_widget(&*textarea, chunks[1 + i]);
        }
    }

    render_status(f, chunks[1 + field_count + 1], &state.status, state.is_error);
    render_footer(
        f,
        chunks[1 + field_count + 2],
        &[("Tab", "next field"), ("Enter", "save"), ("Esc", "cancel")],
    );

    // Render choice picker overlay if open
    if let Some(picker) = &state.choice_picker {
        let popup_area = centered_rect(60, 50, area);
        f.render_widget(Clear, popup_area);
        let popup_block = Block::new()
            .title(" Effort Level ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ORANGE));
        let inner = popup_block.inner(popup_area);
        f.render_widget(popup_block, popup_area);
        picker.list.render(f, inner, true);
    }
}

pub fn handle_key(state: &mut EditState, key: KeyEvent) -> Nav {
    match state.step {
        Step::PickProfile => handle_picker_key(state, key),
        Step::FillForm => handle_form_key(state, key),
    }
}

fn handle_picker_key(state: &mut EditState, key: KeyEvent) -> Nav {
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
                return Nav::AddProfile;
            }
            if let Some(idx) = state.list.selected_original_index() {
                let slug = state.slugs[idx].clone();
                state.init_form(&slug);
                state.step = Step::FillForm;
            }
            Nav::None
        }
        _ => Nav::None,
    }
}

fn handle_form_key(state: &mut EditState, key: KeyEvent) -> Nav {
    let slug = state.current_slug.as_deref().unwrap_or("").to_string();
    let provider_id = match get_profile(&slug) {
        Ok(Some(c)) => c.provider,
        _ => return Nav::Back,
    };
    let provider = match get_provider(&provider_id) {
        Some(p) => p,
        None => return Nav::Back,
    };

    // If a choice picker is open, route keys to it
    if state.choice_picker.is_some() {
        let picker = state.choice_picker.as_mut().unwrap();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                picker.list.move_up();
                return Nav::None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                picker.list.move_down();
                return Nav::None;
            }
            KeyCode::Esc => {
                state.choice_picker = None;
                return Nav::None;
            }
            KeyCode::Enter => {
                if let Some(orig_idx) = picker.list.selected_original_index() {
                    if orig_idx < picker.options.len() {
                        // Predefined option selected
                        let value = picker.options[orig_idx].clone();
                        let ta = &mut state.fields[picker.field_index];
                        ta.select_all();
                        ta.insert_str(&value);
                        state.status = format!("Effort level: {value}");
                        state.is_error = false;
                    }
                    // "Custom..." selected: just close picker, keep current value
                }
                state.choice_picker = None;
                return Nav::None;
            }
            KeyCode::Char(c) if !picker.list.filter.is_empty() || c == '/' => {
                if c != '/' {
                    picker.list.filter.push(c);
                    picker.list.filter_items();
                }
                return Nav::None;
            }
            KeyCode::Backspace => {
                if !picker.list.filter.is_empty() {
                    picker.list.filter.pop();
                    picker.list.filter_items();
                    return Nav::None;
                }
            }
            _ => {
                // All other keys ignored while picker is open
                return Nav::None;
            }
        }
    }

    match key.code {
        KeyCode::Esc => Nav::Back,
        KeyCode::Tab => {
            state.field_cursor = (state.field_cursor + 1) % provider.fields.len();
            Nav::None
        }
        KeyCode::BackTab => {
            if state.field_cursor == 0 {
                state.field_cursor = provider.fields.len() - 1;
            } else {
                state.field_cursor -= 1;
            }
            Nav::None
        }
        KeyCode::Enter => {
            if state.field_cursor + 1 < provider.fields.len() {
                state.field_cursor += 1;
                return Nav::None;
            }
            try_save(state, provider, &slug)
        }
        KeyCode::Char('v') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            if let Some(ta) = state.fields.get_mut(state.field_cursor) {
                ta.paste();
            }
            Nav::None
        }
        KeyCode::Char(' ') => {
            if let Some(field_def) = provider.fields.get(state.field_cursor) {
                // If active field is a Choice, open the picker
                if let FieldType::Choice { options } = &field_def.field_type {
                    let mut items: Vec<(String, String)> = options
                        .iter()
                        .map(|o| (o.to_string(), String::new()))
                        .collect();
                    items.push(("Custom...".to_string(), "type your own value".to_string()));
                    let current_val = state.fields[state.field_cursor].lines().join("").trim().to_string();
                    let selected = if !current_val.is_empty() {
                        options.iter().position(|o| *o == current_val).unwrap_or(items.len() - 1)
                    } else {
                        0
                    };
                    let mut list = SelectList::new(items);
                    list.selected = selected.min(list.filtered_indices.len().saturating_sub(1));
                    state.choice_picker = Some(ChoicePicker {
                        field_index: state.field_cursor,
                        list,
                        options: options.iter().map(|s| s.to_string()).collect(),
                    });
                    return Nav::None;
                }
                // Toggle bool ExtraBody checkbox fields
                if matches!(&field_def.field_type, FieldType::ExtraBody { value_type: ExtraBodyValueType::Bool, .. }) {
                    let ta = &mut state.fields[state.field_cursor];
                    let current = ta.lines().join("").trim().to_lowercase();
                    let is_checked = matches!(current.as_str(), "true" | "yes" | "1");
                    ta.select_all();
                    ta.insert_str(if is_checked { "false" } else { "true" });
                    return Nav::None;
                }
            }
            // Fall through to default text input if not a bool or choice field
            if let Some(ta) = state.fields.get_mut(state.field_cursor) {
                ta.input(key);
            }
            Nav::None
        }
        _ => {
            // Skip text input for bool ExtraBody fields
            let is_bool = provider.fields.get(state.field_cursor)
                .map_or(false, |f| matches!(&f.field_type, FieldType::ExtraBody { value_type: ExtraBodyValueType::Bool, .. }));
            if !is_bool {
                if let Some(ta) = state.fields.get_mut(state.field_cursor) {
                    ta.input(key);
                }
            }
            Nav::None
        }
    }
}

fn try_save(state: &mut EditState, provider: &ProviderDef, old_slug: &str) -> Nav {
    let old_profile = match get_profile(old_slug) {
        Ok(Some(c)) => c,
        _ => return Nav::Back,
    };

    let mut env = std::collections::HashMap::new();
    let mut extra_body = parse_extra_body(None);
    let mut missing_required = None;

    for (i, field_def) in provider.fields.iter().enumerate() {
        let value = state.fields[i].lines().join("").trim().to_string();

        if let FieldType::ExtraBody { json_path, value_type } = &field_def.field_type {
            if !value.is_empty() {
                if let Some(json_value) = serialize_value(value_type, &value) {
                    set_nested_path(&mut extra_body, json_path, json_value);
                }
            }
            continue;
        }

        // For secret and choice fields, if empty keep old value
        let final_value = if value.is_empty() {
            if field_def.field_type == FieldType::Secret || matches!(&field_def.field_type, FieldType::Choice { .. }) {
                old_profile.env.get(field_def.key).cloned().unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            value
        };

        if final_value.is_empty() && field_def.required {
            missing_required = Some(field_def.label);
        } else if !final_value.is_empty() {
            env.insert(field_def.key.to_string(), final_value);
        }
    }

    if let Some(label) = missing_required {
        state.status = format!("Required field missing: {label}");
        state.is_error = true;
        return Nav::None;
    }

    if let Some(body) = build_extra_body(extra_body) {
        env.insert(crate::providers::ENV_EXTRA_BODY.to_string(), body);
    }

    let updated = Profile {
        name: old_profile.name.clone(),
        provider: provider.id.to_string(),
        env,
    };

    match rename_profile(old_slug, old_slug, updated) {
        Ok(_) => {
            state.status = format!("Profile \"{}\" updated!", old_profile.name);
            state.is_error = false;
            Nav::Back
        }
        Err(e) => {
            state.status = format!("Failed to save: {e}");
            state.is_error = true;
            Nav::None
        }
    }
}
