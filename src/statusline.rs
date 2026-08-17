use std::path::PathBuf;

use heed::EnvOpenOptions;
use serde::Deserialize;

use crate::config::get_profile;
use crate::error::AppError;
use crate::providers::{ENV_MANAGEMENT_KEY, ProviderDef, get_provider};

mod deepseek;
mod openrouter;

// ---- Stdin JSON from Claude Code -------------------------------------------

/// Full statusline input sent by Claude Code on stdin.
/// All fields are optional — absent keys default to `None`.
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct StatuslineInput {
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub session_name: Option<String>,
    #[serde(default)]
    pub prompt_id: Option<String>,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub model: Option<ModelInfo>,
    #[serde(default)]
    pub workspace: Option<WorkspaceInfo>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub output_style: Option<OutputStyleInfo>,
    #[serde(default)]
    pub cost: Option<CostInfo>,
    #[serde(default)]
    pub context_window: Option<ContextWindowInfo>,
    #[serde(default)]
    pub exceeds_200k_tokens: Option<bool>,
    #[serde(default)]
    pub effort: Option<EffortInfo>,
    #[serde(default)]
    pub thinking: Option<ThinkingInfo>,
    #[serde(default)]
    pub rate_limits: Option<RateLimitsInfo>,
    #[serde(default)]
    pub vim: Option<VimInfo>,
    #[serde(default)]
    pub agent: Option<AgentInfo>,
    #[serde(default)]
    pub pr: Option<PrInfo>,
    #[serde(default)]
    pub worktree: Option<WorktreeInfo>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ModelInfo {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WorkspaceInfo {
    #[serde(default)]
    pub current_dir: Option<String>,
    #[serde(default)]
    pub project_dir: Option<String>,
    #[serde(default)]
    pub added_dirs: Vec<String>,
    #[serde(default)]
    pub git_worktree: Option<String>,
    #[serde(default)]
    pub repo: Option<RepoInfo>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RepoInfo {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OutputStyleInfo {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CostInfo {
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
    #[serde(default)]
    pub total_duration_ms: Option<u64>,
    #[serde(default)]
    pub total_api_duration_ms: Option<u64>,
    #[serde(default)]
    pub total_lines_added: Option<u64>,
    #[serde(default)]
    pub total_lines_removed: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ContextWindowInfo {
    #[serde(default)]
    pub total_input_tokens: Option<u64>,
    #[serde(default)]
    pub total_output_tokens: Option<u64>,
    #[serde(default)]
    pub context_window_size: Option<u64>,
    #[serde(default)]
    pub used_percentage: Option<f64>,
    #[serde(default)]
    pub remaining_percentage: Option<f64>,
    #[serde(default)]
    pub current_usage: Option<CurrentUsageInfo>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CurrentUsageInfo {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct EffortInfo {
    #[serde(default)]
    pub level: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ThinkingInfo {
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RateLimitsInfo {
    #[serde(default)]
    pub five_hour: Option<RateLimitBucket>,
    #[serde(default)]
    pub seven_day: Option<RateLimitBucket>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RateLimitBucket {
    #[serde(default)]
    pub used_percentage: Option<f64>,
    #[serde(default)]
    pub resets_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VimInfo {
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AgentInfo {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PrInfo {
    #[serde(default)]
    pub number: Option<u64>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub review_state: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WorktreeInfo {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub original_cwd: Option<String>,
    #[serde(default)]
    pub original_branch: Option<String>,
}

// ---- LMDB cache ------------------------------------------------------------

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("claude-launcher")
        .join("statusline-cache")
}

fn open_cache_env() -> Result<heed::Env, AppError> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| AppError::Other(format!("Failed to create cache dir {}: {e}", dir.display())))?;
    unsafe {
        EnvOpenOptions::new()
            .max_dbs(128)
            .open(&dir)
            .map_err(|e| AppError::Other(format!("Failed to open LMDB env: {e}")))
    }
}

// ---- Stdin parsing --------------------------------------------------------

/// Read and parse the stdin JSON from Claude Code.
/// Best-effort: returns `None` on any failure (empty stdin, invalid JSON, etc.).
fn parse_stdin() -> Option<StatuslineInput> {
    let raw = std::io::read_to_string(std::io::stdin()).ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    serde_json::from_str(&raw).ok()
}

struct Session<'p> {
    env: &'p heed::Env,
    provider: &'static ProviderDef,
    session_id: Option<&'p str>,
    token: Option<&'p str>,
    management_key: Option<&'p str>,
}

impl<'p> Session<'p> {
    fn new(
        env: &'p heed::Env,
        provider: &'static ProviderDef,
        session_id: Option<&'p str>,
        token: Option<&'p str>,
        management_key: Option<&'p str>,
    ) -> Result<Self, AppError> {
        Ok(Self {
            env,
            provider,
            session_id,
            token,
            management_key,
        })
    }
}

// ---- Top-level entry point ------------------------------------------------

/// Generate the status line text for the given profile.
///
/// Reads stdin JSON from Claude Code to extract `session_id` for balance caching.
/// Gracefully degrades: if the balance fetch fails, the balance portion shows `--`
/// and the function still returns `Ok(...)` with a valid status string.
pub fn generate_statusline(slug: &str) -> Result<String, AppError> {
    let stdin_input = parse_stdin();
    let session_id = stdin_input.as_ref().and_then(|i| i.session_id.as_deref()).filter(|s| !s.is_empty());

    let profile = get_profile(slug)?.ok_or_else(|| AppError::Other(format!("Profile \"{slug}\" not found")))?;

    let provider = get_provider(&profile.provider).ok_or_else(|| AppError::Other(format!("Unknown provider \"{}\"", profile.provider)))?;

    if !provider.supports_statusline {
        return Err(AppError::Other(format!("Provider \"{}\" does not support status line", provider.name)));
    }

    let token = profile.env.get("ANTHROPIC_AUTH_TOKEN").map(|s| s.as_str());
    let management_key = profile.env.get(ENV_MANAGEMENT_KEY).map(|s| s.as_str());

    let env = open_cache_env()?;
    let session = Session::new(&env, provider, session_id, token, management_key)?;

    match provider.id {
        "deepseek" => deepseek::generate_status_line(session),
        "openrouter" => openrouter::generate_status_line(session),
        _ => Err(AppError::Other(format!("Provider \"{}\" does not support status line", provider.name))),
    }
}
