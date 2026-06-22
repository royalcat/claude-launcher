use std::collections::HashMap;
use std::process::Command;

use crate::config::{get_all_credentials, get_credential};
use crate::error::AppError;

pub fn check_claude_installed() -> bool {
    #[cfg(windows)]
    let probe = "where";
    #[cfg(not(windows))]
    let probe = "which";

    Command::new(probe).arg("claude").output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn build_command(env: &HashMap<String, String>, claude_args: &[String]) -> String {
    fn shell_quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\\''"))
    }

    let args_part = if claude_args.is_empty() {
        String::new()
    } else {
        format!(" {}", claude_args.iter().map(|a| shell_quote(a)).collect::<Vec<_>>().join(" "))
    };

    if env.is_empty() {
        return format!("claude{args_part}");
    }

    let env_parts: Vec<String> = env.iter().map(|(k, v)| format!("{}={}", k, shell_quote(v))).collect();

    format!("{} \\\n  claude{args_part}", env_parts.join(" \\\n  "))
}

/// Spawn `claude` with the given extra env vars and wait for it to exit.
/// Returns the exit code.
pub fn launch_claude(env: &HashMap<String, String>, claude_args: &[String]) -> Result<i32, String> {
    let mut cmd = Command::new("claude");
    cmd.args(claude_args);

    // Inherit parent environment, then overlay provider vars
    let mut full_env: HashMap<String, String> = std::env::vars().collect();
    for (k, v) in env {
        full_env.insert(k.clone(), v.clone());
    }
    cmd.envs(&full_env);

    match cmd.status() {
        Ok(status) => Ok(status.code().unwrap_or(0)),
        Err(e) => Err(e.to_string()),
    }
}

/// Non-interactive launch: look up `slug`, optionally print or spawn.
pub fn launch_with_slug(slug: &str, claude_args: &[String], print_only: bool) -> Result<i32, AppError> {
    let cred = get_credential(slug)?.ok_or_else(|| {
        // Collect available slugs for the error message
        let all = get_all_credentials().unwrap_or_default();
        let available: Vec<String> = all.keys().cloned().collect();
        let hint = if available.is_empty() {
            String::new()
        } else {
            format!(
                "\n  Available credentials:\n{}",
                available
                    .iter()
                    .map(|s| format!("    - {} ({})", s, all[s].name))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        AppError::Other(format!("Credentials \"{slug}\" not found.{hint}"))
    })?;

    if print_only {
        println!("{}", build_command(&cred.env, claude_args));
        return Ok(0);
    }

    if !check_claude_installed() {
        return Err(AppError::Other(
            "\"claude\" not found in PATH.\n  Install Claude Code: https://docs.anthropic.com/en/docs/claude-code".into(),
        ));
    }

    launch_claude(&cred.env, claude_args).map_err(|e| AppError::Other(e))
}
