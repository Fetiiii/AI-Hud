//! Claude Code 5-hour / 7-day usage window.
//!
//! `claude` itself computes these numbers from `anthropic-ratelimit-unified-*`
//! response headers on every inference call, but never persists or exposes
//! them anywhere on disk. Community tooling (see
//! Maciek-roboblog/Claude-Code-Usage-Monitor#202) found the same data is also
//! served directly, for free, by a plain GET on the OAuth session — no
//! inference cost, so it's safe to poll every minute or so.

use super::{ProviderUsage, UsageWindow};
use crate::credentials::read_claude_token;
use crate::state::now_ms;
use anyhow::{bail, Context, Result};
use serde::Deserialize;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

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
            // Anthropic's endpoint names its windows structurally
            // (`five_hour`, `seven_day`, ...) rather than reporting a
            // duration, so there is nothing to derive a label from - the
            // frontend uses the slot's conventional name.
            label: None,
            window_minutes: None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawUsageResponse {
    five_hour: Option<RawWindow>,
    seven_day: Option<RawWindow>,
    seven_day_opus: Option<RawWindow>,
    seven_day_sonnet: Option<RawWindow>,
}

pub async fn fetch(client: &reqwest::Client) -> Result<ProviderUsage> {
    let token = read_claude_token()?;
    if token.expires_at_ms > 0 && token.expires_at_ms < now_ms() as i64 {
        bail!("Claude Code login has expired - run `claude` once to refresh it");
    }

    let resp = client
        .get(USAGE_URL)
        .bearer_auth(&token.access_token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("content-type", "application/json")
        // A generic User-Agent lands in an aggressively rate-limited bucket
        // upstream; mimic the CLI's own UA to stay in the normal one.
        .header("user-agent", "claude-cli/2.0.0 (external, cli)")
        .send()
        .await
        .context("request to Claude usage endpoint failed")?;

    // 429 is not an auth problem, and reporting it as "token expired? run
    // `claude` to re-login" sends the user off to fix something that isn't
    // broken. Name it for what it is, and pass on the server's own hint.
    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .map(|v| format!(" ({v} sn sonra tekrar denenebilir)"))
            .unwrap_or_default();
        bail!("kullanım uç noktası şu an istek sınırında{retry} - eldeki son değer gösteriliyor");
    }

    let resp = resp.error_for_status().context(
        "Claude usage endpoint returned an error status (token expired? run `claude` to re-login)",
    )?;

    let raw: RawUsageResponse = resp
        .json()
        .await
        .context("could not parse Claude usage response as JSON")?;

    Ok(ProviderUsage {
        five_hour: raw.five_hour.map(Into::into),
        seven_day: raw.seven_day.map(Into::into),
        seven_day_opus: raw.seven_day_opus.map(Into::into),
        seven_day_sonnet: raw.seven_day_sonnet.map(Into::into),
        plan_type: None,
    })
}
