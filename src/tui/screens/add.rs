use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use ratatui_textarea::TextArea;

use super::extra_body::{build_extra_body, deserialize_value, get_nested_path, parse_extra_body, serialize_value, set_nested_path};
use super::widgets::{ChoicePicker, SelectList, centered_rect, render_footer, render_status};
use crate::config::{Profile, get_all_profiles, save_profile, slugify_name};
use crate::providers::{ExtraBodyValueType, FieldType, PROVIDERS, ProviderDef, get_provider};
use crate::tui::theme::*;

/// Which sub-step of the Add flow we're in
enum Step {
    PickProvider,
    FillForm,
    PickCopySource,
}

pub struct AddState {
    step: Step,
    provider_list: SelectList,
    /// Set once provider is chosen
    provider_id: Option<String>,
    /// Name field (first in the form)
    name_ta: TextArea<'static>,
    /// Text areas for provider-specific fields
    fields: Vec<TextArea<'static>>,
    /// 0 = name field, 1..=n = provider fields
    field_cursor: usize,
    status: String,
    is_error: bool,
    /// List for picking an existing profile to copy from
    copy_source_list: Option<SelectList>,
    /// Maps list index to profile slug
    copy_source_slugs: Option<Vec<String>>,
    /// When Some, a choice picker popup is open
    choice_picker: Option<ChoicePicker>,
}

impl AddState {
    pub fn new() -> Self {
        let items = PROVIDERS.iter().map(|p| (p.name.to_string(), "".to_string())).collect();
        AddState {
            step: Step::PickProvider,
            provider_list: SelectList::new(items),
            provider_id: None,
            name_ta: TextArea::default(),
            fields: Vec::new(),
            field_cursor: 0,
            status: String::new(),
            is_error: false,
            copy_source_list: None,
            copy_source_slugs: None,
            choice_picker: None,
        }
    }

    fn total_fields(&self) -> usize {
        1 + self.fields.len()
    }

    fn init_form(&mut self, provider: &ProviderDef) {
        self.name_ta = TextArea::default();
        self.fields = provider
            .fields
            .iter()
            .map(|f| {
                let mut ta = TextArea::default();
                if let Some(default) = f.default {
                    ta.insert_str(default);
                }
                ta
            })
            .collect();
        self.field_cursor = 0;
    }

    fn enter_copy_source_picker(&mut self) {
        let Some(provider_id) = &self.provider_id else {
            self.status = "No provider selected".to_string();
            self.is_error = true;
            return;
        };

        let all = match get_all_profiles() {
            Ok(profiles) => profiles,
            Err(_) => {
                self.status = "Failed to load profiles".to_string();
                self.is_error = true;
                return;
            }
        };

        let filtered: Vec<(String, String, String)> = all
            .iter()
            .filter(|(_, c)| c.provider == *provider_id)
            .map(|(slug, c)| {
                let token = c
                    .env
                    .get("ANTHROPIC_AUTH_TOKEN")
                    .map(|t| crate::config::mask_secret(t))
                    .unwrap_or_else(|| "no key".to_string());
                (slug.clone(), c.name.clone(), format!("{} · {}", c.provider, token))
            })
            .collect();

        if filtered.is_empty() {
            self.status = "No existing profiles for this provider".to_string();
            self.is_error = true;
            return;
        }

        let slugs: Vec<String> = filtered.iter().map(|(s, _, _)| s.clone()).collect();
        let items: Vec<(String, String)> = filtered
            .into_iter()
            .map(|(_, name, desc)| (name, desc))
            .collect();

        self.copy_source_slugs = Some(slugs);
        self.copy_source_list = Some(SelectList::new(items));
        self.step = Step::PickCopySource;
    }

    fn copy_fields_from_profile(&mut self, slug: &str) {
        let all = match get_all_profiles() {
            Ok(profiles) => profiles,
            Err(_) => return,
        };

        let Some(profile) = all.get(slug) else {
            return;
        };

        let Some(provider_id) = &self.provider_id else {
            return;
        };

        let Some(provider) = get_provider(provider_id) else {
            return;
        };

        // Parse extra body JSON once if it exists
        let extra_body_json = profile
            .env
            .get(crate::providers::ENV_EXTRA_BODY)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());

        for (field_def, textarea) in provider.fields.iter().zip(self.fields.iter_mut()) {
            match &field_def.field_type {
                FieldType::Url | FieldType::Secret | FieldType::String | FieldType::Choice { .. } => {
                    if let Some(value) = profile.env.get(field_def.key) {
                        textarea.select_all();
                        textarea.insert_str(value);
                    }
                }
                FieldType::ExtraBody { json_path, value_type } => {
                    if let Some(ref json) = extra_body_json {
                        if let Some(value) = get_nested_path(json, json_path) {
                            if let Some(display) = deserialize_value(value_type, value) {
                                textarea.select_all();
                                textarea.insert_str(&display);
                            }
                        }
                    }
                }
            }
        }
        // Intentionally NOT modifying name_ta — user must enter a new name
    }

    fn handle_copy_source_key(&mut self, key: KeyEvent) -> bool {
        let Some(list) = &mut self.copy_source_list else {
            return false;
        };

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                list.move_up();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                list.move_down();
                true
            }
            KeyCode::Enter => {
                let mut slug_to_copy: Option<String> = None;
                if let Some(orig_idx) = list.selected_original_index() {
                    if let Some(slugs) = &self.copy_source_slugs {
                        if let Some(slug) = slugs.get(orig_idx) {
                            slug_to_copy = Some(slug.clone());
                        }
                    }
                }
                if let Some(slug) = slug_to_copy {
                    self.copy_fields_from_profile(&slug);
                    self.status = "Fields copied from existing profile".to_string();
                    self.is_error = false;
                }
                self.copy_source_list = None;
                self.copy_source_slugs = None;
                self.step = Step::FillForm;
                true
            }
            KeyCode::Esc => {
                self.copy_source_list = None;
                self.copy_source_slugs = None;
                self.step = Step::FillForm;
                true
            }
            KeyCode::Char(c) if !list.filter.is_empty() || c == '/' => {
                if c != '/' {
                    list.filter.push(c);
                    list.filter_items();
                }
                true
            }
            KeyCode::Backspace => {
                if !list.filter.is_empty() {
                    list.filter.pop();
                    list.filter_items();
                }
                true
            }
            _ => false,
        }
    }
}

pub enum Nav {
    None,
    Back,
}

pub fn render(f: &mut Frame, state: &mut AddState) {
    let area = f.area();

    match state.step {
        Step::PickProvider => render_provider_picker(f, state, area),
        Step::FillForm => render_form(f, state, area),
        Step::PickCopySource => render_copy_source_picker(f, state, area),
    }
}

fn render_provider_picker(f: &mut Frame, state: &mut AddState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(8), Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("Choose a provider:", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        dim("(/ to filter)"),
    ]));
    f.render_widget(title, chunks[0]);

    state.provider_list.render(f, chunks[1], true);
    render_status(f, chunks[2], &state.status, state.is_error);
    render_footer(f, chunks[3], &[("↑↓", "move"), ("Enter", "select"), ("Esc", "back")]);
}

fn render_form(f: &mut Frame, state: &mut AddState, area: Rect) {
    let provider_id = state.provider_id.as_deref().unwrap_or("");
    let provider = match get_provider(provider_id) {
        Some(p) => p,
        None => return,
    };

    let title_text = format!("Add profile — {}", provider.name);
    let total = state.total_fields();

    // Build constraints: title + name field + one block per provider field + spacer + status + footer
    let mut constraints = vec![Constraint::Length(2), Constraint::Length(3)];
    for _ in &state.fields {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Min(1));
    constraints.push(Constraint::Length(2));
    constraints.push(Constraint::Length(2));

    let chunks = Layout::default().direction(Direction::Vertical).constraints(constraints).split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(title_text, Style::default().add_modifier(Modifier::BOLD)),
    ]));
    f.render_widget(title, chunks[0]);

    // Render name field at chunk[1]
    {
        let is_active = state.field_cursor == 0;
        let border_style = if is_active {
            Style::default().fg(ORANGE)
        } else {
            Style::default().fg(DIM_COLOR)
        };
        let block = Block::default()
            .title(Span::styled("  Name *", border_style))
            .borders(Borders::ALL)
            .border_style(border_style);
        state.name_ta.set_block(block);
        if is_active {
            state.name_ta.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        } else {
            state.name_ta.set_cursor_style(Style::default());
        }
        f.render_widget(&state.name_ta, chunks[1]);
    }

    // Render provider fields at chunks[2..]
    for (i, (field_def, textarea)) in provider.fields.iter().zip(state.fields.iter_mut()).enumerate() {
        let is_active = state.field_cursor == i + 1;
        let req_marker = if field_def.required { " *" } else { " (opt)" };
        let border_style = if is_active {
            Style::default().fg(ORANGE)
        } else {
            Style::default().fg(DIM_COLOR)
        };
        let is_choice = matches!(&field_def.field_type, FieldType::Choice { .. });
        let is_bool = matches!(
            &field_def.field_type,
            FieldType::ExtraBody { value_type: ExtraBodyValueType::Bool, .. }
        );

        if is_choice {
            // Render as a selectable value with (Space to pick) hint
            let hint = if is_active { " (Space to pick)" } else { "" };
            let label = format!("  {} {}{}", field_def.label, req_marker, hint);
            let raw = textarea.lines().join("").trim().to_string();
            let display = if raw.is_empty() { "[select]" } else { &raw };
            let block = Block::default()
                .title(Span::styled(label, border_style))
                .borders(Borders::ALL)
                .border_style(border_style);
            let inner = block.inner(chunks[2 + i]);
            f.render_widget(block, chunks[2 + i]);
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
            let label = format!("  {} {}", field_def.label, req_marker);
            let raw = textarea.lines().join("").trim().to_lowercase();
            let is_checked = matches!(raw.as_str(), "true" | "yes" | "1");
            let checkbox_text = if is_checked { "[x] Yes" } else { "[ ] No" };
            let block = Block::default()
                .title(Span::styled(label, border_style))
                .borders(Borders::ALL)
                .border_style(border_style);
            let inner = block.inner(chunks[2 + i]);
            f.render_widget(block, chunks[2 + i]);
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
            let label = format!("  {} {}", field_def.label, req_marker);
            let block = Block::default()
                .title(Span::styled(label, border_style))
                .borders(Borders::ALL)
                .border_style(border_style);
            textarea.set_block(block);

            use crate::providers::FieldType;
            if field_def.field_type == FieldType::Secret {
                textarea.set_mask_char('\u{2022}');
            }

            if is_active {
                textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
            } else {
                textarea.set_cursor_style(Style::default());
            }

            f.render_widget(&*textarea, chunks[2 + i]);
        }
    }

    render_status(f, chunks[1 + total + 1], &state.status, state.is_error);
    render_footer(f, chunks[1 + total + 2], &[("Tab", "next field"), ("Enter", "save"), ("Esc", "cancel"), ("Ctrl+R", "copy from existing")]);

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

fn render_copy_source_picker(f: &mut Frame, state: &mut AddState, area: Rect) {
    let Some(list) = &mut state.copy_source_list else {
        return;
    };

    // Create a centered popup (60% width, 50% height)
    let popup_area = centered_rect(60, 50, area);

    // Clear the popup area
    f.render_widget(Clear, popup_area);

    let block = Block::new()
        .title(" Copy from Existing Profile ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    list.render(f, inner, true);
}

pub fn handle_key(state: &mut AddState, key: KeyEvent) -> Nav {
    match state.step {
        Step::PickProvider => handle_provider_key(state, key),
        Step::FillForm => handle_form_key(state, key),
        Step::PickCopySource => {
            if state.handle_copy_source_key(key) {
                Nav::None
            } else {
                Nav::None
            }
        }
    }
}

fn handle_provider_key(state: &mut AddState, key: KeyEvent) -> Nav {
    match key.code {
        KeyCode::Esc => {
            if !state.provider_list.filter.is_empty() {
                state.provider_list.filter.clear();
                state.provider_list.filter_items();
                return Nav::None;
            }
            return Nav::Back;
        }
        KeyCode::Char(c) if !state.provider_list.filter.is_empty() || c == '/' => {
            if c != '/' {
                state.provider_list.filter.push(c);
                state.provider_list.filter_items();
            }
            return Nav::None;
        }
        KeyCode::Backspace => {
            if !state.provider_list.filter.is_empty() {
                state.provider_list.filter.pop();
                state.provider_list.filter_items();
                return Nav::None;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.provider_list.move_up();
            return Nav::None;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.provider_list.move_down();
            return Nav::None;
        }
        KeyCode::Enter => {
            if let Some(idx) = state.provider_list.selected_original_index() {
                let provider = &PROVIDERS[idx];
                state.provider_id = Some(provider.id.to_string());
                state.init_form(provider);
                state.step = Step::FillForm;
                state.status.clear();
            }
            return Nav::None;
        }
        _ => {}
    }
    Nav::None
}

fn handle_form_key(state: &mut AddState, key: KeyEvent) -> Nav {
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
        KeyCode::Esc => {
            return Nav::Back;
        }
        KeyCode::Tab => {
            state.field_cursor = (state.field_cursor + 1) % state.total_fields();
            return Nav::None;
        }
        KeyCode::BackTab => {
            let total = state.total_fields();
            if state.field_cursor == 0 {
                state.field_cursor = total - 1;
            } else {
                state.field_cursor -= 1;
            }
            return Nav::None;
        }
        KeyCode::Enter => {
            let provider_id = state.provider_id.as_deref().unwrap_or("");
            let provider = match get_provider(provider_id) {
                Some(p) => p,
                None => return Nav::Back,
            };
            if state.field_cursor + 1 < state.total_fields() {
                state.field_cursor += 1;
                return Nav::None;
            }
            return try_save(state, provider);
        }
        KeyCode::Char('r') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            state.enter_copy_source_picker();
            return Nav::None;
        }
        KeyCode::Char('v') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            if state.field_cursor == 0 {
                state.name_ta.paste();
            } else if let Some(ta) = state.fields.get_mut(state.field_cursor - 1) {
                ta.paste();
            }
            return Nav::None;
        }
        KeyCode::Char(' ') => {
            let provider_id = state.provider_id.as_deref().unwrap_or("");
            if let Some(provider) = get_provider(provider_id) {
                if state.field_cursor > 0 {
                    let field_idx = state.field_cursor - 1;
                    if let Some(field_def) = provider.fields.get(field_idx) {
                        // If active field is a Choice, open the picker
                        if let FieldType::Choice { options } = &field_def.field_type {
                            let mut items: Vec<(String, String)> = options
                                .iter()
                                .map(|o| (o.to_string(), String::new()))
                                .collect();
                            items.push(("Custom...".to_string(), "type your own value".to_string()));
                            let current_val = state.fields[field_idx].lines().join("").trim().to_string();
                            let selected = if !current_val.is_empty() {
                                options.iter().position(|o| *o == current_val).unwrap_or(items.len() - 1)
                            } else {
                                0
                            };
                            let mut list = SelectList::new(items);
                            list.selected = selected.min(list.filtered_indices.len().saturating_sub(1));
                            state.choice_picker = Some(ChoicePicker {
                                field_index: field_idx,
                                list,
                                options: options.iter().map(|s| s.to_string()).collect(),
                            });
                            return Nav::None;
                        }
                        // Toggle bool ExtraBody checkbox fields
                        if matches!(&field_def.field_type, FieldType::ExtraBody { value_type: ExtraBodyValueType::Bool, .. }) {
                            let ta = &mut state.fields[field_idx];
                            let current = ta.lines().join("").trim().to_lowercase();
                            let is_checked = matches!(current.as_str(), "true" | "yes" | "1");
                            ta.select_all();
                            ta.insert_str(if is_checked { "false" } else { "true" });
                            return Nav::None;
                        }
                    }
                }
            }
        }
        _ => {}
    }

    // Forward input to the active textarea (skip bool ExtraBody fields)
    if state.field_cursor == 0 {
        state.name_ta.input(key);
    } else if let Some(ta) = state.fields.get_mut(state.field_cursor - 1) {
        let is_bool = state.provider_id.as_deref()
            .and_then(|id| get_provider(id))
            .and_then(|p| p.fields.get(state.field_cursor - 1))
            .map_or(false, |f| matches!(&f.field_type, FieldType::ExtraBody { value_type: ExtraBodyValueType::Bool, .. }));
        if !is_bool {
            ta.input(key);
        }
    }
    Nav::None
}

fn try_save(state: &mut AddState, provider: &ProviderDef) -> Nav {
    // Get the user-provided name
    let name = state.name_ta.lines().join("").trim().to_string();
    if name.is_empty() {
        state.status = "Name is required".to_string();
        state.is_error = true;
        state.field_cursor = 0;
        return Nav::None;
    }

    // Collect env vars; ExtraBody fields are merged into one JSON object
    let mut env = std::collections::HashMap::new();
    let mut extra_body = parse_extra_body(None);
    let mut missing_required = None;

    for (i, field_def) in provider.fields.iter().enumerate() {
        let value = state.fields[i].lines().join("").trim().to_string();
        if value.is_empty() {
            if field_def.required {
                missing_required = Some(field_def.label);
            }
            continue;
        }
        if let FieldType::ExtraBody { json_path, value_type } = &field_def.field_type {
            if let Some(json_value) = serialize_value(value_type, &value) {
                set_nested_path(&mut extra_body, json_path, json_value);
            }
        } else {
            env.insert(field_def.key.to_string(), value);
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

    // Check for slug collision
    let slug = slugify_name(&name);
    if slug.is_empty() {
        state.status = "Name produces an empty slug — use at least one alphanumeric character".to_string();
        state.is_error = true;
        return Nav::None;
    }

    let all = get_all_profiles().unwrap_or_default();
    let final_slug = if all.contains_key(&slug) {
        let mut i = 2;
        loop {
            let candidate = format!("{slug}-{i}");
            if !all.contains_key(&candidate) {
                break candidate;
            }
            i += 1;
        }
    } else {
        slug
    };

    let profile = Profile {
        name: name.clone(),
        provider: provider.id.to_string(),
        env,
    };

    match save_profile(&final_slug, profile) {
        Ok(_) => {
            state.status = format!("Profile \"{name}\" saved as \"{final_slug}\"!");
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
