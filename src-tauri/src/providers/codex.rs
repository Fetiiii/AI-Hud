//! Codex CLI 5-hour / weekly usage window.
//!
//! UNVERIFIED / BEST EFFORT: community reverse-engineering points at a
//! `/wham/usage`-style endpoint behind the ChatGPT backend, requiring a
//! `ChatGPT-Account-Id` header alongside the bearer token (see
//! steipete/CodexBar and openai/codex#15281). This was written without a
//! real Codex account to test against, so treat the URL/headers/response
//! shape below as a starting point to confirm and correct against an actual
//! `~/.codex/auth.json` + live response before trusting the numbers.

use super::{ProviderUsage, UsageWindow};
use crate::credentials::read_codex_token;
use anyhow::{Context, Result};
use serde::Deserialize;

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

#[derive(Debug, Default, Deserialize)]
struct RawWindow {
    #[serde(default)]
    utilization: f64,
    #[serde(default)]
    resets_at: Option<String>,
}

impl From<RawWindow> for UsageWindow {
    fn from(w: RawWindow) -> Self {
        UsageWindow {
            utilization_pct: w.utilization,
            resets_at: w.resets_at,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawUsageResponse {
    five_hour: Option<RawWindow>,
    seven_day: Option<RawWindow>,
}

pub async fn fetch(client: &reqwest::Client) -> Result<ProviderUsage> {
    let token = read_codex_token()?;

    let mut req = client
        .get(USAGE_URL)
        .bearer_auth(&token.access_token)
        .header("content-type", "application/json");

    if let Some(account_id) = &token.account_id {
        req = req.header("chatgpt-account-id", account_id);
    }

    let resp = req
        .send()
        .await
        .context("request to Codex usage endpoint failed")?
        .error_for_status()
        .context("Codex usage endpoint returned an error status (schema/endpoint unconfirmed - see module docs)")?;

    let raw: RawUsageResponse = resp
        .json()
        .await
        .context("could not parse Codex usage response as JSON (schema unconfirmed)")?;

    Ok(ProviderUsage {
        five_hour: raw.five_hour.map(Into::into),
        seven_day: raw.seven_day.map(Into::into),
        seven_day_opus: None,
        seven_day_sonnet: None,
    })
}
