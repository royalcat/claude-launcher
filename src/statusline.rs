use std::path::PathBuf;

use heed::types::{SerdeBincode, Str};
use heed::{Database, EnvOpenOptions};
use jiff::Timestamp;
use jiff::tz::{Offset, TimeZone};
use serde::{Deserialize, Serialize};

use crate::config::get_profile;
use crate::error::AppError;
use crate::providers::get_provider;

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

// ---- DeepSeek balance API types -------------------------------------------

#[derive(Debug, Deserialize)]
struct BalanceResponse {
    #[allow(dead_code)]
    is_available: bool,
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Debug, Deserialize)]
struct BalanceInfo {
    #[allow(dead_code)]
    currency: String,
    total_balance: String,
    #[allow(dead_code)]
    #[serde(default)]
    granted_balance: String,
    #[allow(dead_code)]
    #[serde(default)]
    topped_up_balance: String,
}

// ---- LMDB cache ------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    balance: String,
    timestamp: i64,
}

const CACHE_TTL_SECS: i64 = 60;

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("claude-launcher")
        .join("statusline-cache")
}

fn open_cache_env() -> Result<heed::Env, AppError> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| AppError::Other(format!("Failed to create cache dir {}: {e}", dir.display())))?;
    // SAFETY: we use a dedicated directory with max_dbs(1), no unsafe flags.
    // The environment is opened for a single process — LMDB handles concurrent
    // readers across invocations through its file locking.
    unsafe {
        EnvOpenOptions::new()
            .max_dbs(1)
            .open(&dir)
            .map_err(|e| AppError::Other(format!("Failed to open LMDB env: {e}")))
    }
}

fn cached_balance(session_id: &str) -> Option<String> {
    let env = open_cache_env().ok()?;
    let rtxn = env.read_txn().ok()?;

    // Try to open existing database, or return None if it doesn't exist yet
    let db: Database<Str, SerdeBincode<CacheEntry>> = match env.open_database(&rtxn, None).ok()? {
        Some(db) => db,
        None => return None,
    };

    let entry = db.get(&rtxn, session_id).ok()??;

    let now = Timestamp::now().as_second();
    if now - entry.timestamp <= CACHE_TTL_SECS {
        Some(entry.balance)
    } else {
        None
    }
}

fn store_balance(session_id: &str, balance: &str) {
    let env = match open_cache_env() {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut wtxn = match env.write_txn() {
        Ok(t) => t,
        Err(_) => return,
    };

    // Open existing DB or create it
    let db: Database<Str, SerdeBincode<CacheEntry>> = match env.open_database(&wtxn, None).ok().flatten() {
        Some(db) => db,
        None => match env.create_database(&mut wtxn, None) {
            Ok(db) => db,
            Err(_) => return,
        },
    };

    let entry = CacheEntry {
        balance: balance.to_string(),
        timestamp: Timestamp::now().as_second(),
    };
    let _ = db.put(&mut wtxn, session_id, &entry);
    let _ = wtxn.commit();
}

// ---- Balance fetching -----------------------------------------------------

/// Fetch the DeepSeek account balance via their API.
/// Returns the total balance as a formatted string (e.g. "110.00") or an error message.
pub fn fetch_balance(api_key: &str) -> Result<String, String> {
    let resp = ureq::get("https://api.deepseek.com/user/balance")
        .header("Authorization", &format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?
        .body_mut()
        .read_json::<BalanceResponse>()
        .map_err(|e| format!("Failed to parse balance response: {}", e))?;

    if !resp.is_available {
        return Ok("Balance unavailable".to_string());
    }

    // Prefer CNY balance, fall back to first available
    // let info = resp
    //     .balance_infos
    //     .iter()
    //     .find(|b| b.currency == "CNY")
    //     .or_else(|| resp.balance_infos.first())
    //     .ok_or_else(|| "No balance information in response".to_string())?;

    // Ok(info.total_balance.clone())

    Ok(resp
        .balance_infos
        .iter()
        .map(|b| format!("{} {}", b.total_balance, b.currency))
        .collect::<Vec<_>>()
        .join(" "))
}

// ---- Peak hours -----------------------------------------------------------

/// Returns true if the current Beijing time (UTC+8) falls within peak hours.
///
/// Peak hours are currently hardcoded as 09:00–12:00 and 14:00–18:00.
///
/// TODO: Fetch peak hours from DeepSeek API when such an endpoint becomes available.
pub fn is_peak_hours() -> bool {
    // Beijing time via IANA timezone (China Standard Time, no DST)
    let hour = Timestamp::now()
        .to_zoned(TimeZone::get("Asia/Shanghai").unwrap_or(TimeZone::fixed(Offset::from_hours(8).unwrap())))
        .datetime()
        .hour();

    (hour >= 9 && hour < 12) || (hour >= 14 && hour < 18)
}

// ---- Formatting -----------------------------------------------------------

/// Format the status line text for display in Claude Code's status bar.
pub fn format_statusline(provider_name: &str, balance: Option<&str>, peak: bool) -> String {
    format!(
        "{provider_name} | Balance: {} | {}",
        balance.unwrap_or("--"),
        if peak { "Peak" } else { "Off-Peak" }
    )
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

    let token = profile.env.get("ANTHROPIC_AUTH_TOKEN");

    let balance = match (token, session_id) {
        // With session_id: check cache first, then fetch, then store
        (Some(t), Some(sid)) => {
            if let Some(cached) = cached_balance(sid) {
                Some(cached)
            } else if let Ok(b) = fetch_balance(t) {
                store_balance(sid, &b);
                Some(b)
            } else {
                // Fetch failed — try stale cache as fallback
                None
            }
        }
        // No session_id: uncached fetch
        (Some(t), None) => fetch_balance(t).ok(),
        // No token: can't fetch
        _ => None,
    };

    let peak = is_peak_hours();

    Ok(format_statusline(provider.name, balance.as_deref(), peak))
}
