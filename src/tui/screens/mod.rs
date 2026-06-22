pub mod add;
pub mod delete;
pub mod edit;
pub mod extra_body;
pub mod help;
pub mod launch;
pub mod main_menu;
pub mod settings;
pub mod widgets;

use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::tui::App;

/// Navigation signals from screen handlers
pub enum Action {
    Continue,
    Quit,
    LaunchClaude {
        /// None = launch without a profile (default)
        slug: Option<String>,
        claude_args: Vec<String>,
        print_only: bool,
    },
}

/// Top-level screen discriminant
pub enum Screen {
    MainMenu(main_menu::MainMenuState),
    Launch(launch::LaunchState),
    Add(add::AddState),
    Edit(edit::EditState),
    Delete(delete::DeleteState),
    Settings(settings::SettingsState),
    Help,
}

pub fn render(f: &mut Frame, app: &mut App) {
    match &mut app.screen {
        Screen::MainMenu(state) => {
            let last = crate::settings::get_last_launched_profile();
            main_menu::render(f, state, &app.workspace_label, &app.workspace_path, &last)
        }
        Screen::Launch(state) => launch::render(f, state),
        Screen::Add(state) => add::render(f, state),
        Screen::Edit(state) => edit::render(f, state),
        Screen::Delete(state) => delete::render(f, state),
        Screen::Settings(state) => settings::render(f, state),
        Screen::Help => help::render(f),
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Action {
    match &mut app.screen {
        Screen::MainMenu(state) => {
            let nav = main_menu::handle_key(state, key);
            match nav {
                main_menu::Nav::None => Action::Continue,
                main_menu::Nav::Exit => Action::Quit,
                main_menu::Nav::GoTo(screen) => {
                    app.screen = screen;
                    Action::Continue
                }
                main_menu::Nav::LaunchDefault => Action::LaunchClaude {
                    slug: None,
                    claude_args: vec![],
                    print_only: false,
                },
                main_menu::Nav::LaunchLast => {
                    if let Some(slug) = crate::settings::get_last_launched_profile() {
                        Action::LaunchClaude {
                            slug: Some(slug),
                            claude_args: vec![],
                            print_only: false,
                        }
                    } else {
                        Action::Continue
                    }
                }
            }
        }
        Screen::Launch(state) => {
            let nav = launch::handle_key(state, key);
            match nav {
                launch::Nav::None => Action::Continue,
                launch::Nav::Back => {
                    app.refresh_workspace();
                    let last = crate::settings::get_last_launched_profile();
                    app.screen = Screen::MainMenu(main_menu::MainMenuState::new(last));
                    Action::Continue
                }
                launch::Nav::Launch { slug, claude_args } => {
                    crate::settings::update_last_launched_profile(&slug);
                    Action::LaunchClaude {
                        slug: Some(slug),
                        claude_args,
                        print_only: false,
                    }
                }
                launch::Nav::AddProfile => {
                    app.screen = Screen::Add(add::AddState::new());
                    Action::Continue
                }
            }
        }
        Screen::Add(state) => {
            let nav = add::handle_key(state, key);
            match nav {
                add::Nav::None => Action::Continue,
                add::Nav::Back => {
                    app.refresh_workspace();
                    let last = crate::settings::get_last_launched_profile();
                    app.screen = Screen::MainMenu(main_menu::MainMenuState::new(last));
                    Action::Continue
                }
            }
        }
        Screen::Edit(state) => {
            let nav = edit::handle_key(state, key);
            match nav {
                edit::Nav::None => Action::Continue,
                edit::Nav::Back => {
                    app.refresh_workspace();
                    let last = crate::settings::get_last_launched_profile();
                    app.screen = Screen::MainMenu(main_menu::MainMenuState::new(last));
                    Action::Continue
                }
                edit::Nav::AddProfile => {
                    app.screen = Screen::Add(add::AddState::new());
                    Action::Continue
                }
            }
        }
        Screen::Delete(state) => {
            let nav = delete::handle_key(state, key);
            match nav {
                delete::Nav::None => Action::Continue,
                delete::Nav::Back => {
                    app.refresh_workspace();
                    let last = crate::settings::get_last_launched_profile();
                    app.screen = Screen::MainMenu(main_menu::MainMenuState::new(last));
                    Action::Continue
                }
                delete::Nav::AddProfile => {
                    app.screen = Screen::Add(add::AddState::new());
                    Action::Continue
                }
            }
        }
        Screen::Settings(state) => {
            let nav = settings::handle_key(state, key);
            match nav {
                settings::Nav::None => Action::Continue,
                settings::Nav::Back => {
                    app.refresh_workspace();
                    let last = crate::settings::get_last_launched_profile();
                    app.screen = Screen::MainMenu(main_menu::MainMenuState::new(last));
                    Action::Continue
                }
            }
        }
        Screen::Help => {
            let nav = help::handle_key(key);
            match nav {
                help::Nav::Back => {
                    let last = crate::settings::get_last_launched_profile();
                    app.screen = Screen::MainMenu(main_menu::MainMenuState::new(last));
                    Action::Continue
                }
                help::Nav::None => Action::Continue,
            }
        }
    }
}
