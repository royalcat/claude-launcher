// ---- OpenRouter key API types ---------------------------------------------

use heed::types::{SerdeBincode, Str};
use serde::{Deserialize, Serialize};

use crate::{error::AppError, statusline::Session};

#[derive(Clone, Debug, Deserialize)]
struct KeyResponse {
    data: KeyData,
}

#[derive(Clone, Debug, Deserialize)]
struct KeyData {
    #[serde(default)]
    limit_remaining: Option<f64>,
    #[serde(default)]
    usage: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct CreditsResponse {
    data: CreditsData,
}

#[derive(Clone, Debug, Deserialize)]
struct CreditsData {
    #[serde(default)]
    total_credits: f64,
    #[serde(default)]
    total_usage: f64,
}

// ---- Balance fetching -----------------------------------------------------

/// Fetch the OpenRouter account info via their key API.
/// Returns the pre-formatted detail string: the remaining spending limit
/// (e.g. "Balance: $74.50") or, for pay-as-you-go accounts without a limit,
/// the total credit usage (e.g. "Used: $25.50").
pub fn fetch_balance(api_key: &str) -> Result<String, String> {
    let resp = ureq::get("https://openrouter.ai/api/v1/key")
        .header("Authorization", &format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?
        .body_mut()
        .read_json::<KeyResponse>()
        .map_err(|e| format!("Failed to parse key response: {}", e))?;

    match resp.data.limit_remaining {
        Some(remaining) => Ok(format!("Balance: ${:.2}", remaining)),
        None => Ok(format!("Used: ${:.2}", resp.data.usage)),
    }
}

/// Fetch the account-wide credit balance via the management-key endpoint.
/// Returns e.g. "Balance: $74.75" (total credits purchased minus total usage).
pub fn fetch_credits(management_key: &str) -> Result<String, String> {
    let resp = ureq::get("https://openrouter.ai/api/v1/credits")
        .header("Authorization", &format!("Bearer {}", management_key))
        .header("Accept", "application/json")
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?
        .body_mut()
        .read_json::<CreditsResponse>()
        .map_err(|e| format!("Failed to parse credits response: {}", e))?;

    Ok(format!("Balance: ${:.2}", resp.data.total_credits - resp.data.total_usage))
}

// Cache

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    balance: String,
    timestamp: i64,
}

const CACHE_TTL_SECS: i64 = 60;

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

fn cached_balance(txn: &heed::RwTxn, cache: &heed::Database<Str, SerdeBincode<CacheEntry>>, session_id: &str) -> Option<String> {
    let entry = cache.get(&txn, session_id).ok()??;

    let now = now_secs();
    if now - entry.timestamp <= CACHE_TTL_SECS {
        Some(entry.balance)
    } else {
        None
    }
}

fn store_balance(txn: &mut heed::RwTxn, cache: &heed::Database<Str, SerdeBincode<CacheEntry>>, session_id: &str, balance: &str) {
    let entry = CacheEntry {
        balance: balance.to_string(),
        timestamp: now_secs(),
    };
    let _ = cache.put(txn, session_id, &entry);
}

pub fn generate_status_line(session: Session) -> Result<String, crate::statusline::AppError> {
    let mut txn = session.env.write_txn().map_err(|e| AppError::Other(e.to_string()))?;
    let cache = session
        .env
        .create_database(&mut txn, Some(session.provider.id))
        .map_err(|e| AppError::Other(e.to_string()))?;

    // Prefer the management-key account balance; fall back to the regular key.
    let fetch = || -> Option<String> {
        if let Some(mgmt) = session.management_key
            && let Ok(b) = fetch_credits(mgmt)
        {
            return Some(b);
        }
        session.token.and_then(|t| fetch_balance(t).ok())
    };

    let balance = match session.session_id {
        // With session_id: check cache first, then fetch, then store
        Some(sid) if session.token.is_some() || session.management_key.is_some() => {
            if let Some(cached) = cached_balance(&txn, &cache, sid) {
                Some(cached)
            } else if let Some(b) = fetch() {
                store_balance(&mut txn, &cache, sid, &b);
                Some(b)
            } else {
                None
            }
        }
        // No session_id: uncached fetch
        None => fetch(),
        // No credentials: can't fetch
        _ => None,
    };

    Ok(format_statusline(session.provider.name, balance.as_deref()))
}

/// Format the status line text for display in Claude Code's status bar.
pub fn format_statusline(provider_name: &str, detail: Option<&str>) -> String {
    format!("{provider_name} | {}", detail.unwrap_or("Balance: --"))
}
