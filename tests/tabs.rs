//! Integration tests for the grouped-field tabs on the OpenRouter provider form.
//!
//! These drive the *public* Add/Edit screen APIs (AddState/EditState + handle_key +
//! render) against a fixed-size `TestBackend`, so no real terminal or crossterm raw
//! mode is involved. They verify:
//!   - the tab bar renders for the grouped OpenRouter provider,
//!   - the tab auto-derives from the focused field (crossing from General to Provider
//!     Selection flips the rendered field set),
//!   - a non-grouped provider (zai) renders flat, without a tab bar,
//!   - the Edit form shows the same tab bar for a saved OpenRouter profile.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, Terminal, backend::TestBackend};

use claude_launcher::tui::screens::{add, edit};

/// Render a closure to a fixed-size TestBackend and return the frame as lines.
fn draw_to_lines<F>(width: u16, height: u16, draw: F) -> Vec<String>
where
    F: FnOnce(&mut Frame),
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(draw).expect("tui render must not panic");
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect::<String>())
        .collect()
}

fn text_of(lines: &[String]) -> String {
    lines.join("\n")
}

/// Navigate the Add provider picker to a provider by index and select it.
fn select_provider(state: &mut add::AddState, index: usize) {
    for _ in 0..index {
        add::handle_key(state, KeyEvent::from(KeyCode::Down));
    }
    add::handle_key(state, KeyEvent::from(KeyCode::Enter));
}

/// Advance the Add form cursor with Tab `count` times.
fn tab(state: &mut add::AddState, count: usize) {
    for _ in 0..count {
        add::handle_key(state, KeyEvent::from(KeyCode::Tab));
    }
}

#[test]
fn openrouter_shows_tab_bar_and_general_tab_first() {
    let mut state = add::AddState::new();
    select_provider(&mut state, 2); // openrouter
    let text = text_of(&draw_to_lines(80, 50, |f| add::render(f, &mut state)));

    // Tab bar present with both labels.
    assert!(text.contains("General"), "tab bar should show General:\n{text}");
    assert!(text.contains("Provider Selection"), "tab bar should show Provider Selection:\n{text}");

    // Active tab (General) fields rendered; Provider Selection fields hidden.
    assert!(text.contains("API Base URL"), "General field missing:\n{text}");
    assert!(text.contains("API Key"), "General field missing:\n{text}");
    assert!(
        !text.contains("Provider Only"),
        "Provider Selection field should be hidden on the General tab:\n{text}"
    );
    assert!(
        !text.contains("Allow Fallbacks"),
        "Provider Selection field should be hidden on the General tab:\n{text}"
    );
}

#[test]
fn moving_focus_to_provider_selection_flips_the_tab() {
    let mut state = add::AddState::new();
    select_provider(&mut state, 2); // openrouter
    // Cursor starts at the name field (cursor 0). Field index 6 = "Provider Only" lives in
    // the Provider Selection group, so advancing to cursor 7 must auto-flip the tab.
    tab(&mut state, 7);
    let text = text_of(&draw_to_lines(80, 50, |f| add::render(f, &mut state)));

    assert!(text.contains("Provider Only"), "Provider Selection field should now be visible:\n{text}");
    assert!(
        text.contains("Quantization Levels"),
        "Provider Selection field should now be visible:\n{text}"
    );
    assert!(
        !text.contains("API Base URL"),
        "General field should be hidden on the Provider Selection tab:\n{text}"
    );
}

#[test]
fn backtab_returns_to_general_tab() {
    let mut state = add::AddState::new();
    select_provider(&mut state, 2); // openrouter
    tab(&mut state, 7); // into Provider Selection
    add::handle_key(&mut state, KeyEvent::from(KeyCode::BackTab)); // back to cursor 6 (Opus Override)
    let text = text_of(&draw_to_lines(80, 50, |f| add::render(f, &mut state)));

    assert!(text.contains("Opus Model Override"), "General field should be visible again:\n{text}");
    assert!(
        !text.contains("Provider Only"),
        "Provider Selection field should be hidden again:\n{text}"
    );
}

#[test]
fn non_grouped_provider_renders_flat_without_tabs() {
    let mut state = add::AddState::new();
    select_provider(&mut state, 1); // zai
    let text = text_of(&draw_to_lines(80, 50, |f| add::render(f, &mut state)));

    assert!(
        !text.contains("Provider Selection"),
        "zai has no groups; tab bar should be absent:\n{text}"
    );
    assert!(!text.contains("General"), "zai has no groups; tab bar should be absent:\n{text}");
    assert!(text.contains("API Key"), "flat form should render fields:\n{text}");
    assert!(text.contains("Effort Level"), "flat form should render all fields:\n{text}");
}

#[test]
fn edit_openrouter_profile_shows_tab_bar() {
    use claude_launcher::settings;

    // Point the app at a temp config containing one OpenRouter profile, so the test is
    // hermetic and does not read or touch the user's real profiles file.
    let dir = std::env::temp_dir();
    let path = dir.join(format!("claude-launcher-tab-test-{}.json", std::process::id()));
    let cfg = r#"{
        "profiles": {
            "test-or": {
                "name": "Test OR",
                "provider": "openrouter",
                "env": {
                    "ANTHROPIC_BASE_URL": "https://openrouter.ai/api",
                    "ANTHROPIC_AUTH_TOKEN": "sk-test"
                },
                "statusline_enabled": true
            }
        }
    }"#;
    std::fs::write(&path, cfg).unwrap();
    settings::set_runtime_config_path(path.to_string_lossy().to_string());

    let mut state = edit::EditState::new();
    // Select the single profile in the picker.
    edit::handle_key(&mut state, KeyEvent::from(KeyCode::Enter));

    let text = text_of(&draw_to_lines(80, 50, |f| edit::render(f, &mut state)));
    assert!(text.contains("General"), "Edit tab bar should show General:\n{text}");
    assert!(
        text.contains("Provider Selection"),
        "Edit tab bar should show Provider Selection:\n{text}"
    );
    assert!(text.contains("API Base URL"), "Edit General field missing:\n{text}");
    assert!(
        !text.contains("Provider Only"),
        "Edit Provider Selection field should be hidden on General tab:\n{text}"
    );

    // Cross into Provider Selection: Edit cursor 6 = field 6 ("Provider Only").
    for _ in 0..6 {
        edit::handle_key(&mut state, KeyEvent::from(KeyCode::Tab));
    }
    let text = text_of(&draw_to_lines(80, 50, |f| edit::render(f, &mut state)));
    assert!(text.contains("Provider Only"), "Edit should flip to Provider Selection fields:\n{text}");
    assert!(
        !text.contains("API Base URL"),
        "Edit General field should be hidden on Provider Selection tab:\n{text}"
    );

    let _ = std::fs::remove_file(&path);
}
