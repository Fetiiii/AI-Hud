//! Codex CLI 5-hour / weekly usage window.
//!
//! Confirmed against steipete/CodexBar's open-source implementation
//! (`Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthUsageFetcher.swift`)
//! and a real `~/.codex/auth.json` sample: `GET
//! https://chatgpt.com/backend-api/wham/usage` with a bearer token and an
//! optional `ChatGPT-Account-Id` header. The response nests
//! `rate_limit.primary_window` (5h) / `rate_limit.secondary_window` (weekly)
//! as `{ used_percent, reset_at, limit_window_seconds }`, where `reset_at` is
//! a Unix timestamp in seconds (unlike Claude's ISO-8601 string).

use super::{ProviderUsage, UsageWindow};
use crate::credentials::read_codex_token;
use anyhow::{Context, Result};
use serde::Deserialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

#[derive(Debug, Deserialize)]
struct RawWindow {
    used_percent: f64,
    reset_at: i64,
}

#[derive(Debug, Default, Deserialize)]
struct RawRateLimit {
    primary_window: Option<RawWindow>,
    secondary_window: Option<RawWindow>,
}

#[derive(Debug, Default, Deserialize)]
struct RawUsageResponse {
    rate_limit: Option<RawRateLimit>,
}

fn unix_to_rfc3339(secs: i64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(secs)
        .ok()?
        .format(&Rfc3339)
        .ok()
}

impl From<RawWindow> for UsageWindow {
    fn from(w: RawWindow) -> Self {
        UsageWindow {
            utilization_pct: w.used_percent,
            resets_at: unix_to_rfc3339(w.reset_at),
        }
    }
}

pub async fn fetch(client: &reqwest::Client) -> Result<ProviderUsage> {
    let token = read_codex_token()?;

    let mut req = client
        .get(USAGE_URL)
        .bearer_auth(&token.access_token)
        .header("accept", "application/json")
        .header("user-agent", "ai-hud/0.1.0 (+https://github.com/Fetiiii/AI-Hud)");

    if let Some(account_id) = &token.account_id {
        req = req.header("chatgpt-account-id", account_id);
    }

    let resp = req
        .send()
        .await
        .context("request to Codex usage endpoint failed")?
        .error_for_status()
        .context("Codex usage endpoint returned an error status (token expired? run `codex` to re-login)")?;

    let raw: RawUsageResponse = resp
        .json()
        .await
        .context("could not parse Codex usage response as JSON")?;

    let rate_limit = raw.rate_limit.unwrap_or_default();
    Ok(ProviderUsage {
        five_hour: rate_limit.primary_window.map(Into::into),
        seven_day: rate_limit.secondary_window.map(Into::into),
        seven_day_opus: None,
        seven_day_sonnet: None,
    })
}
