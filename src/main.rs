mod actions;
mod cli;
mod config;
mod error;
mod providers;
mod settings;
mod tui;

use clap::Parser;
use cli::{Cli, Command};
use error::AppError;
use settings::{expand_path, list_workspaces, set_runtime_config_path};

fn report_fatal(err: &AppError) -> ! {
    match err {
        AppError::Corrupt(e) => {
            eprintln!("\n  {e}");
            eprintln!("  Fix the JSON by hand, or delete the file to start fresh.\n");
        }
        AppError::Access(e) => {
            eprintln!("\n  {e}");
            eprintln!("  Check file permissions and try again.\n");
        }
        AppError::Other(msg) => {
            eprintln!("\n  Error: {msg}\n");
        }
    }
    std::process::exit(1);
}

fn apply_overrides(cli: &Cli) {
    if cli.workspace.is_some() && cli.config.is_some() {
        eprintln!("\n  --workspace and --config are mutually exclusive — pick one.\n");
        std::process::exit(1);
    }
    if let Some(ref config_path) = cli.config {
        set_runtime_config_path(expand_path(config_path));
        return;
    }
    if let Some(ref workspace) = cli.workspace {
        let workspaces = list_workspaces();
        if !workspaces.contains_key(workspace.as_str()) {
            let available = workspaces.keys().cloned().collect::<Vec<_>>().join(", ");
            let available = if available.is_empty() { "(none)".to_string() } else { available };
            eprintln!("\n  Unknown workspace: \"{workspace}\"");
            eprintln!("  Available workspaces: {available}\n");
            std::process::exit(1);
        }
        set_runtime_config_path(workspaces[workspace.as_str()].clone());
    }
}

fn main() {
    let cli = Cli::parse();
    apply_overrides(&cli);

    // Non-interactive: --credentials <slug> (legacy)
    if let Some(ref slug) = cli.credentials {
        match actions::launch::launch_with_slug(slug, &cli.claude_args, cli.print) {
            Ok(code) => std::process::exit(code),
            Err(e) => report_fatal(&e),
        }
    }

    match cli.command {
        Some(Command::List) => {
            if let Err(e) = actions::list::list_credentials() {
                report_fatal(&e);
            }
        }

        Some(Command::Launch { slug: Some(slug), print }) => {
            let print_only = print || cli.print;
            match actions::launch::launch_with_slug(&slug, &cli.claude_args, print_only) {
                Ok(code) => std::process::exit(code),
                Err(e) => report_fatal(&e),
            }
        }

        Some(Command::Launch { slug: None, .. }) => {
            // Interactive launch: enter TUI with launch screen pre-selected
            if let Err(e) = tui::run() {
                report_fatal(&e);
            }
        }

        None => {
            // Interactive: main TUI menu
            if let Err(e) = tui::run() {
                report_fatal(&e);
            }
        }
    }
}
