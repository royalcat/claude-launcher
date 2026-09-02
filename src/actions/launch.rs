use std::collections::HashMap;
use std::process::Command;

use crate::config::{Profile, get_all_profiles, get_profile};
use crate::error::AppError;
use crate::providers::get_provider;

pub fn check_claude_installed() -> bool {
    #[cfg(windows)]
    let probe = "where";
    #[cfg(not(windows))]
    let probe = "which";

    Command::new(probe).arg("claude").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Env vars that are launcher-internal (e.g. used by the statusline fetch)
/// and must never be exported to the launched `claude` process or printed.
const INTERNAL_ENV_KEYS: &[&str] = &[crate::providers::ENV_MANAGEMENT_KEY];

fn external_env(env: &HashMap<String, String>) -> HashMap<String, String> {
    env.iter()
        .filter(|(k, _)| !INTERNAL_ENV_KEYS.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// For `llama-single-model` profiles, auto-detect the served model name by
/// querying `/v1/models` and inject it into the three Claude model env vars
/// at launch time. All other providers return the stored env unchanged.
fn resolve_llama_env(slug: &str, profile: &Profile) -> Result<HashMap<String, String>, AppError> {
    if profile.provider != crate::providers::PROVIDER_LLAMACPP_SINGLE_MODEL {
        return Ok(profile.env.clone());
    }

    let base_url = profile.env.get(crate::providers::ENV_BASE_URL).map(|s| s.as_str()).unwrap_or_default();
    if base_url.trim().is_empty() {
        return Err(AppError::Other(format!(
            "Profile \"{slug}\" has no ANTHROPIC_BASE_URL — cannot auto-detect the llama.cpp model."
        )));
    }

    let auth_token = profile.env.get(crate::providers::ENV_AUTH_TOKEN).map(|s| s.as_str()).unwrap_or("");

    let model = crate::providers::llamacpp::detect_model(base_url, auth_token)
        .map_err(|e| AppError::Other(format!("Failed to auto-detect model for profile \"{slug}\":\n  {e}")))?;

    let mut env = profile.env.clone();
    env.insert(crate::providers::ENV_HAIKU_MODEL.to_string(), model.clone());
    env.insert(crate::providers::ENV_SONNET_MODEL.to_string(), model.clone());
    env.insert(crate::providers::ENV_OPUS_MODEL.to_string(), model);
    Ok(env)
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

/// Prepend `--settings` with a custom statusLine command to `claude_args`
/// when the profile has statusline enabled and the provider supports it.
fn build_statusline_args(slug: &str, profile: &Profile, claude_args: &[String]) -> Vec<String> {
    if !profile.statusline_enabled {
        return claude_args.to_vec();
    }
    let provider_supports = get_provider(&profile.provider).map(|p| p.supports_statusline).unwrap_or(false);
    if !provider_supports {
        return claude_args.to_vec();
    }

    // Resolve absolute path to this binary for robustness, so the statusline
    // command works regardless of PATH.
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "claude-launcher".to_string());

    let settings_json = serde_json::json!({
        "statusLine": {
            "type": "command",
            "command": format!("{exe} statusline --profile {slug}")
        }
    })
    .to_string();

    let mut args = vec!["--settings".to_string(), settings_json];
    args.extend_from_slice(claude_args);
    args
}

/// Non-interactive launch: look up `slug`, optionally print or spawn.
pub fn launch_with_slug(slug: &str, claude_args: &[String], print_only: bool) -> Result<i32, AppError> {
    let profile = get_profile(slug)?.ok_or_else(|| {
        // Collect available slugs for the error message
        let all = get_all_profiles().unwrap_or_default();
        let available: Vec<String> = all.keys().cloned().collect();
        let hint = if available.is_empty() {
            String::new()
        } else {
            format!(
                "\n  Available profiles:\n{}",
                available
                    .iter()
                    .map(|s| format!("    - {} ({})", s, all[s].name))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        AppError::Other(format!("Profile \"{slug}\" not found.{hint}"))
    })?;

    // Auto-detect and inject the model for llama-single-model profiles.
    let resolved_env = resolve_llama_env(slug, &profile)?;

    // Augment args with --settings if statusline is enabled
    let augmented_args = build_statusline_args(slug, &profile, claude_args);

    if print_only {
        println!("{}", build_command(&external_env(&resolved_env), &augmented_args));
        return Ok(0);
    }

    if !check_claude_installed() {
        return Err(AppError::Other(
            "\"claude\" not found in PATH.\n  Install Claude Code: https://docs.anthropic.com/en/docs/claude-code".into(),
        ));
    }

    launch_claude(&external_env(&resolved_env), &augmented_args).map_err(|e| AppError::Other(e))
}
