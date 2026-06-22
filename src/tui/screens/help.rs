use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::widgets::render_footer;
use crate::providers::PROVIDERS;
use crate::settings::{get_active_workspace, get_config_path, list_workspaces, settings_path};
use crate::tui::theme::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub enum Nav {
    None,
    Back,
}

pub fn render(f: &mut Frame) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(area);

    let (active_label, _) = get_active_workspace();
    let workspaces = list_workspaces();
    let profiles_path = get_config_path();
    let settings_file = settings_path().to_string_lossy().into_owned();

    let mut lines: Vec<Line> = vec![
        Line::raw(""),
        Line::from(vec![Span::raw("  "), orange("claude-launcher"), dim(&format!("  ·  v{VERSION}"))]),
        Line::from(dim("  ─────────────────────────────────────────")),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("What is this?", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(Span::raw("  A CLI tool to manage multiple sets of Claude Code profiles")),
        Line::from(Span::raw("  and launch Claude Code with any of them instantly.")),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Quick start", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("   "),
            dim("1.  "),
            Span::styled("Add profile", Style::default().add_modifier(Modifier::BOLD)),
            dim("        → pick a provider, paste your API key"),
        ]),
        Line::from(vec![
            Span::raw("   "),
            dim("2.  "),
            Span::styled("Launch with provider", Style::default().add_modifier(Modifier::BOLD)),
            dim("   → run Claude Code with that profile"),
        ]),
        Line::raw(""),
        Line::from(vec![Span::raw("  "), Span::styled("Keys", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(vec![
            Span::raw("   "),
            Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
            dim(" move  ·  "),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            dim(" select  ·  "),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            dim(" back  ·  "),
            Span::styled("Ctrl-C", Style::default().add_modifier(Modifier::BOLD)),
            dim(" quit"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Commands", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("   "),
            dim("$ "),
            Span::raw("claude-launcher"),
            dim("                          # interactive TUI"),
        ]),
        Line::from(vec![
            Span::raw("   "),
            dim("$ "),
            Span::raw("claude-launcher list"),
            dim("                   # list all profiles"),
        ]),
        Line::from(vec![
            Span::raw("   "),
            dim("$ "),
            Span::raw("claude-launcher launch"),
            dim("                # pick a profile interactively"),
        ]),
        Line::from(vec![
            Span::raw("   "),
            dim("$ "),
            Span::raw("claude-launcher launch <slug>"),
            dim("          # launch with a specific profile"),
        ]),
        Line::from(vec![
            Span::raw("   "),
            dim("$ "),
            Span::raw("claude-launcher launch <slug> --print"),
            dim("  # show env vars only"),
        ]),
        Line::from(vec![
            Span::raw("   "),
            dim("$ "),
            Span::raw("claude-launcher launch <slug> -- <args>"),
            dim(" # pass args to claude"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Providers", Style::default().add_modifier(Modifier::BOLD)),
        ]),
    ];

    for p in PROVIDERS {
        lines.push(Line::from(vec![Span::raw("   "), Span::raw(p.name)]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Workspaces", Style::default().add_modifier(Modifier::BOLD)),
    ]));

    let mut sorted_workspaces: Vec<(String, String)> = workspaces.into_iter().collect();
    sorted_workspaces.sort_by(|a, b| a.0.cmp(&b.0));
    for (label, path) in &sorted_workspaces {
        let is_active = label == &active_label;
        let marker = if is_active {
            Span::styled(" (active)", Style::default().fg(GREEN))
        } else {
            Span::raw("")
        };
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(label.clone(), Style::default().add_modifier(Modifier::BOLD)),
            marker,
            dim(&format!("  {path}")),
        ]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Files", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![Span::raw("   "), Span::raw(settings_file), dim("  (settings)")]));
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::raw(profiles_path),
        dim("  (profiles — mode 0600)"),
    ]));
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Project", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled("https://github.com/royalcat/claude-launcher", Style::default().fg(CYAN)),
    ]));
    lines.push(Line::raw(""));

    let p = Paragraph::new(lines);
    f.render_widget(p, chunks[0]);

    render_footer(f, chunks[1], &[("Esc", "back"), ("q", "back")]);
}

pub fn handle_key(key: KeyEvent) -> Nav {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => Nav::Back,
        _ => Nav::None,
    }
}
