use aomi_sdk::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

// ── App struct ────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
pub(crate) struct FanForgeApp;

// ── HTTP helpers ──────────────────────────────────────────────────────────────

const ZORA_API_BASE: &str = "https://api-sdk.zora.engineering";

pub(crate) fn zora_get(path_with_query: &str) -> Result<Value, String> {
    let url = format!("{ZORA_API_BASE}{path_with_query}");
    let api_key = std::env::var("ZORA_API_KEY").ok();

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("[fanforge] failed to build HTTP client: {e}"))?;

    let mut req = client.get(&url);
    if let Some(k) = &api_key {
        req = req.header("api-key", k);
    }

    let resp = req
        .send()
        .map_err(|e| format!("[fanforge] Zora GET {path_with_query} failed: {e}"))?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "[fanforge] Zora API {path_with_query} returned {status}: {}",
            &body[..body.len().min(300)]
        ));
    }

    serde_json::from_str(&body)
        .map_err(|e| format!("[fanforge] Zora response decode failed: {e}"))
}

pub(crate) fn zora_post(path: &str, body: &Value) -> Result<Value, String> {
    let url = format!("{ZORA_API_BASE}{path}");
    let api_key = std::env::var("ZORA_API_KEY").ok();

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("[fanforge] failed to build HTTP client: {e}"))?;

    let mut req = client.post(&url).json(body);
    if let Some(k) = &api_key {
        req = req.header("api-key", k);
    }

    let resp = req
        .send()
        .map_err(|e| format!("[fanforge] Zora POST {path} failed: {e}"))?;

    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "[fanforge] Zora POST {path} returned {status}: {}",
            &text[..text.len().min(300)]
        ));
    }

    serde_json::from_str(&text)
        .map_err(|e| format!("[fanforge] Zora POST decode failed: {e}"))
}

/// Minimal percent-encoder for URL query values.
pub(crate) fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{:02X}", b);
        }
    }
    out
}

// ── Supabase REST helpers (state for missions / distributions) ────────────────

fn supabase_headers() -> Result<(String, String), String> {
    let url = std::env::var("SUPABASE_URL")
        .map_err(|_| "[fanforge] SUPABASE_URL not set".to_string())?;
    let key = std::env::var("SUPABASE_ANON_KEY")
        .map_err(|_| "[fanforge] SUPABASE_ANON_KEY not set".to_string())?;
    Ok((url, key))
}

pub(crate) fn supabase_get(table: &str, query: &str) -> Result<Value, String> {
    let (url, key) = supabase_headers()?;
    let endpoint = format!("{url}/rest/v1/{table}?{query}");

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("[fanforge] HTTP client: {e}"))?;

    let resp = client
        .get(&endpoint)
        .header("apikey", &key)
        .header("Authorization", format!("Bearer {key}"))
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("[fanforge] Supabase GET failed: {e}"))?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "[fanforge] Supabase GET {table} returned {status}: {}",
            &body[..body.len().min(300)]
        ));
    }

    serde_json::from_str(&body).map_err(|e| format!("[fanforge] Supabase decode: {e}"))
}

pub(crate) fn supabase_post(table: &str, body: &Value) -> Result<Value, String> {
    let (url, key) = supabase_headers()?;
    let endpoint = format!("{url}/rest/v1/{table}");

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("[fanforge] HTTP client: {e}"))?;

    let resp = client
        .post(&endpoint)
        .header("apikey", &key)
        .header("Authorization", format!("Bearer {key}"))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=representation")
        .json(body)
        .send()
        .map_err(|e| format!("[fanforge] Supabase POST failed: {e}"))?;

    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "[fanforge] Supabase POST {table} returned {status}: {}",
            &text[..text.len().min(300)]
        ));
    }

    serde_json::from_str(&text).map_err(|e| format!("[fanforge] Supabase POST decode: {e}"))
}

// ── Args structs (shared across tool.rs) ─────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct LaunchFanCoinArgs {
    /// Creator's Telegram ID — used to link the coin to their profile.
    pub creator_telegram_id: String,
    /// Display name of the fan coin (e.g. "Temi's Fan Coin").
    pub name: String,
    /// Ticker symbol — 3 to 5 uppercase letters (e.g. "TEMI").
    pub ticker: String,
    /// Plain-English description of the coin and what it means for fans.
    pub description: String,
    /// Optional IPFS or HTTPS URL for the coin's cover art.
    #[serde(default)]
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetFanLeaderboardArgs {
    /// Zora coin contract address on Base (0x...).
    pub coin_address: String,
    /// Maximum number of holders to return. Defaults to 10, max 50.
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CreateFanMissionArgs {
    /// Zora coin contract address — the coin holders must hold to qualify.
    pub coin_address: String,
    /// Short title for the mission (e.g. "Early Access: Unreleased Track").
    pub title: String,
    /// The exclusive content link to deliver to qualifying fans.
    pub content_url: String,
    /// Minimum coin balance required to unlock the mission.
    pub threshold: f64,
    /// Optional ISO-8601 expiry date. Defaults to 30 days from now.
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DistributeRewardsArgs {
    /// Mission ID returned by `fanforge_create_fan_mission`.
    pub mission_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetCreatorRecapArgs {
    /// Zora coin contract address.
    pub coin_address: String,
    /// Number of days to look back. Defaults to 7.
    #[serde(default)]
    pub days: Option<u32>,
}

// ── Shared response helper ────────────────────────────────────────────────────

pub(crate) fn ok<T: Serialize>(value: T) -> Result<serde_json::Value, String> {
    let value = serde_json::to_value(value).map_err(|e| format!("[fanforge] serialize: {e}"))?;
    Ok(match value {
        serde_json::Value::Object(mut m) => {
            m.insert("source".into(), serde_json::Value::String("fanforge".into()));
            serde_json::Value::Object(m)
        }
        other => serde_json::json!({ "source": "fanforge", "data": other }),
    })
}
