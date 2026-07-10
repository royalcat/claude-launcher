use crate::error::AppError;
use crate::statusline;

pub fn print_statusline(slug: &str) -> Result<(), AppError> {
    match statusline::generate_statusline(slug) {
        Ok(text) => {
            println!("{text}");
            Ok(())
        }
        Err(e) => {
            // Always print something to stdout so Claude Code's status bar
            // isn't blank. The actual error goes to stderr.
            eprintln!("Statusline error: {e}");
            println!("Error");
            Ok(())
        }
    }
}
