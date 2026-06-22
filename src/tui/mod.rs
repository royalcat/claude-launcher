pub mod screens;
pub mod theme;

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::error::AppError;
use crate::settings::get_active_profile;

use screens::Screen;

pub struct App {
    pub screen: Screen,
    pub profile_label: String,
    pub profile_path: String,
}

impl App {
    pub fn new() -> Self {
        let (label, path) = get_active_profile();
        App {
            screen: Screen::MainMenu(screens::main_menu::MainMenuState::new(
                crate::settings::get_last_launched_credential(),
            )),
            profile_label: label,
            profile_path: path,
        }
    }

    pub fn refresh_profile(&mut self) {
        let (label, path) = get_active_profile();
        self.profile_label = label;
        self.profile_path = path;
    }
}

pub fn run() -> Result<(), AppError> {
    enable_raw_mode().map_err(|e| AppError::Other(e.to_string()))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| AppError::Other(e.to_string()))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| AppError::Other(e.to_string()))?;

    let result = run_app(&mut terminal);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), AppError> {
    let mut app = App::new();

    loop {
        terminal
            .draw(|f| screens::render(f, &mut app))
            .map_err(|e| AppError::Other(e.to_string()))?;

        if event::poll(std::time::Duration::from_millis(100)).map_err(|e| AppError::Other(e.to_string()))? {
            match event::read().map_err(|e| AppError::Other(e.to_string()))? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    // Global Ctrl-C → quit
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                        return Ok(());
                    }

                    match screens::handle_key(&mut app, key) {
                        screens::Action::Quit => return Ok(()),
                        screens::Action::LaunchClaude {
                            slug,
                            claude_args,
                            print_only,
                        } => {
                            // Exit TUI before launching
                            disable_raw_mode().ok();
                            execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
                            terminal.show_cursor().ok();

                            if let Some(slug) = slug {
                                match crate::actions::launch::launch_with_slug(&slug, &claude_args, print_only) {
                                    Ok(code) => std::process::exit(code),
                                    Err(e) => {
                                        eprintln!("\n  Error: {e}\n");
                                        std::process::exit(1);
                                    }
                                }
                            } else {
                                // Launch default (no env overrides)
                                match crate::actions::launch::launch_claude(&Default::default(), &claude_args) {
                                    Ok(code) => std::process::exit(code),
                                    Err(e) => {
                                        eprintln!("\n  Error: {e}\n");
                                        std::process::exit(1);
                                    }
                                }
                            }
                        }
                        screens::Action::Continue => {}
                    }
                }
                _ => {}
            }
        }
    }
}
