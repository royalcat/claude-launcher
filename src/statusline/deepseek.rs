// ---- DeepSeek balance API types -------------------------------------------

use crossterm::style::Stylize;
use heed::types::{SerdeBincode, Str};
use itertools::Itertools;
use jiff::{
    Timestamp,
    tz::{Offset, TimeZone},
};
use serde::{Deserialize, Serialize};

use crate::{error::AppError, statusline::Session};

#[derive(Clone, Debug, Deserialize)]
struct BalanceResponse {
    #[allow(dead_code)]
    is_available: bool,
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Clone, Debug, Deserialize)]
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

    Ok(resp
        .balance_infos
        .into_iter()
        .filter(|b| b.total_balance.parse::<f64>().unwrap_or(1.0) > 0.01)
        .sorted_by(|a, b| a.currency.cmp(&b.currency))
        .map(|b| {
            let currency = iso_currency::Currency::from_code(&b.currency)
                .map(|c| c.symbol().to_string())
                .unwrap_or(b.currency);
            format!("{} {}", b.total_balance, currency)
        })
        .join(" + "))
}

// Cache

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    balance: String,
    timestamp: i64,
}

const CACHE_TTL_SECS: i64 = 60;

fn cached_balance(txn: &heed::RwTxn, cache: &heed::Database<Str, SerdeBincode<CacheEntry>>, session_id: &str) -> Option<String> {
    let entry = cache.get(&txn, session_id).ok()??;

    let now = Timestamp::now().as_second();
    if now - entry.timestamp <= CACHE_TTL_SECS {
        Some(entry.balance)
    } else {
        None
    }
}

fn store_balance(txn: &mut heed::RwTxn, cache: &heed::Database<Str, SerdeBincode<CacheEntry>>, session_id: &str, balance: &str) {
    let entry = CacheEntry {
        balance: balance.to_string(),
        timestamp: Timestamp::now().as_second(),
    };
    let _ = cache.put(txn, session_id, &entry);
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

pub fn generate_status_line(session: Session) -> Result<String, crate::statusline::AppError> {
    let mut txn = session.env.write_txn().map_err(|e| AppError::Other(e.to_string()))?;
    let cache = session
        .env
        .create_database(&mut txn, Some(session.provider.id))
        .map_err(|e| AppError::Other(e.to_string()))?;

    let balance = match (session.token, session.session_id) {
        // With session_id: check cache first, then fetch, then store
        (Some(t), Some(sid)) => {
            if let Some(cached) = cached_balance(&txn, &cache, sid) {
                Some(cached)
            } else if let Ok(b) = fetch_balance(t) {
                store_balance(&mut txn, &cache, sid, &b);
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

    Ok(format_statusline(session.provider.name, balance.as_deref(), peak))
}

/// Format the status line text for display in Claude Code's status bar.
pub fn format_statusline(provider_name: &str, balance: Option<&str>, peak: bool) -> String {
    format!(
        "{provider_name} | Balance: {} | {}",
        balance.unwrap_or("--"),
        if peak { "Peak".stylize().red() } else { "Off-Peak".stylize() }
    )
}
