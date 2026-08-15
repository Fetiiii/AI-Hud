//! Codex CLI usage windows.
//!
//! Two sources, deliberately:
//!
//! 1. **The local rollout log** (`~/.codex/sessions/**/rollout-*.jsonl`).
//!    Codex writes a `rate_limits` block into `token_count` events on every
//!    turn, carrying `used_percent`, `window_minutes`, `resets_at` (unix
//!    seconds), `plan_type` and `credits`. No network, no token expiry, and
//!    it can't break when an undocumented endpoint changes shape. Its one
//!    weakness is staleness: it only updates when Codex is actually used.
//!
//! 2. **`GET https://chatgpt.com/backend-api/wham/usage`** with the bearer
//!    token from `~/.codex/auth.json`. Schema confirmed against
//!    steipete/CodexBar. Fresher, but undocumented and auth-dependent.
//!
//! We read both and keep whichever is fresher, so the HUD still shows real
//! numbers when the endpoint 401s and still updates when Codex sits idle.
//!
//! Note the two sources spell the same field differently - the rollout log
//! says `window_minutes` / `resets_at`, the endpoint says
//! `limit_window_seconds` / `reset_at` - so they get separate parsers rather
//! than one lenient struct that would silently mislabel one of them.

use super::{window_label, ProviderUsage, UsageWindow};
use crate::credentials::read_codex_token;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

/// How many recent rollout files to look through before giving up. The newest
/// session often has no `rate_limits` yet (it is only written once a turn
/// completes), so we need to look back a little - but not across the whole
/// history, which grows without bound.
const ROLLOUT_SCAN_LIMIT: usize = 12;

fn unix_to_rfc3339(secs: i64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(secs)
        .ok()?
        .format(&Rfc3339)
        .ok()
}

// ---------------------------------------------------------------------------
// Network source: GET /backend-api/wham/usage
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RawWindow {
    used_percent: f64,
    reset_at: i64,
    /// Present on the endpoint but previously dropped, which is what made a
    /// monthly free-plan window render as "5 saatlik".
    #[serde(default)]
    limit_window_seconds: Option<u64>,
}

impl From<RawWindow> for UsageWindow {
    fn from(w: RawWindow) -> Self {
        let minutes = w.limit_window_seconds.map(|s| s / 60);
        UsageWindow {
            utilization_pct: w.used_percent,
            resets_at: unix_to_rfc3339(w.reset_at),
            label: minutes.map(window_label),
            window_minutes: minutes,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawRateLimit {
    primary_window: Option<RawWindow>,
    secondary_window: Option<RawWindow>,
}

#[derive(Debug, Default, Deserialize)]
struct RawUsageResponse {
    rate_limit: Option<RawRateLimit>,
    #[serde(default)]
    plan_type: Option<String>,
}

async fn fetch_remote(client: &reqwest::Client) -> Result<ProviderUsage> {
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
        plan_type: raw.plan_type,
    })
}

// ---------------------------------------------------------------------------
// Local source: ~/.codex/sessions/**/rollout-*.jsonl
// ---------------------------------------------------------------------------

/// The `rate_limits` block as it appears inside a rollout log's `token_count`
/// event. Field names differ from the endpoint's - see the module docs.
#[derive(Debug, Deserialize)]
struct LocalWindow {
    used_percent: f64,
    #[serde(default)]
    window_minutes: Option<u64>,
    #[serde(default)]
    resets_at: Option<i64>,
}

impl From<LocalWindow> for UsageWindow {
    fn from(w: LocalWindow) -> Self {
        UsageWindow {
            utilization_pct: w.used_percent,
            resets_at: w.resets_at.and_then(unix_to_rfc3339),
            label: w.window_minutes.map(window_label),
            window_minutes: w.window_minutes,
        }
    }
}

#[derive(Debug, Deserialize)]
struct LocalRateLimits {
    #[serde(default)]
    primary: Option<LocalWindow>,
    #[serde(default)]
    secondary: Option<LocalWindow>,
    #[serde(default)]
    plan_type: Option<String>,
}

/// The most recently modified rollout files, newest first.
fn recent_rollouts(root: &Path, limit: usize) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<(SystemTime, PathBuf)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
                continue;
            };
            out.push((modified, path));
        }
    }

    let mut found = Vec::new();
    walk(root, &mut found);
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().take(limit).map(|(_, p)| p).collect()
}

/// Pulls the newest `rate_limits` block out of a single rollout file.
/// Returns `None` for files that have none yet (a session that hasn't
/// completed a turn writes `token_count` events with `rate_limits: null`).
fn rate_limits_in(path: &Path) -> Option<(String, LocalRateLimits)> {
    let raw = std::fs::read_to_string(path).ok()?;
    for line in raw.lines().rev() {
        let line = line.trim();
        if line.is_empty() || !line.contains("\"rate_limits\"") {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(node) = value.pointer("/payload/rate_limits") else {
            continue;
        };
        let Ok(parsed) = serde_json::from_value::<LocalRateLimits>(node.clone()) else {
            continue;
        };
        // A block with both windows null carries no usage - keep scanning.
        if parsed.primary.is_none() && parsed.secondary.is_none() {
            continue;
        }
        let timestamp = value
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        return Some((timestamp, parsed));
    }
    None
}

/// Reads usage straight off disk. `Ok(None)` means Codex has simply never
/// recorded a rate limit here (fresh install, or no completed turn yet).
pub fn local_usage() -> Result<Option<ProviderUsage>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("no home dir"))?;
    let sessions = home.join(".codex").join("sessions");
    if !sessions.exists() {
        return Ok(None);
    }

    let mut best: Option<(String, LocalRateLimits)> = None;
    for path in recent_rollouts(&sessions, ROLLOUT_SCAN_LIMIT) {
        let Some((timestamp, limits)) = rate_limits_in(&path) else {
            continue;
        };
        // Files are visited newest-modified first, but a file's mtime can run
        // ahead of its last rate_limits event, so compare event timestamps.
        if best.as_ref().map(|(t, _)| timestamp > *t).unwrap_or(true) {
            best = Some((timestamp, limits));
        }
    }

    let Some((_, limits)) = best else {
        return Ok(None);
    };

    Ok(Some(ProviderUsage {
        five_hour: limits.primary.map(Into::into),
        seven_day: limits.secondary.map(Into::into),
        seven_day_opus: None,
        seven_day_sonnet: None,
        plan_type: limits.plan_type,
    }))
}

// ---------------------------------------------------------------------------

/// How much of a window's period has elapsed, used to decide which of the two
/// sources is fresher: the one whose reset is further out saw a newer window.
fn resets_at_key(usage: &ProviderUsage) -> Option<&str> {
    usage
        .five_hour
        .as_ref()
        .or(usage.seven_day.as_ref())
        .and_then(|w| w.resets_at.as_deref())
}

/// Local first, network to refresh. Whichever reports the later reset time
/// wins, so a stale cached endpoint response can't clobber a rollout written
/// seconds ago (and vice versa). An endpoint failure is not fatal as long as
/// the local log has something.
pub async fn fetch(client: &reqwest::Client) -> Result<ProviderUsage> {
    let local = local_usage().unwrap_or(None);
    let remote = fetch_remote(client).await;

    match (local, remote) {
        (Some(local), Ok(remote)) => {
            let keep_remote = match (resets_at_key(&local), resets_at_key(&remote)) {
                (Some(l), Some(r)) => r >= l,
                // A source that reports no reset time at all is the weaker one.
                (Some(_), None) => false,
                _ => true,
            };
            Ok(if keep_remote { remote } else { local })
        }
        (Some(local), Err(_)) => Ok(local),
        (None, Ok(remote)) => Ok(remote),
        (None, Err(e)) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shape as described by a real `~/.codex/sessions/**/rollout-*.jsonl`:
    /// `session_meta` first, then arbitrary events, with `token_count`
    /// `event_msg`s scattered throughout (only the last one should win).
    fn sample_rollout() -> String {
        [
            r#"{"timestamp":"2026-08-14T13:54:20Z","ordinal":0,"type":"session_meta","payload":{"session_id":"019fffe8","cwd":"/home/user/some-project","originator":"codex-tui"}}"#,
            // rate_limits present but empty - must not win over the later one.
            r#"{"timestamp":"2026-08-14T13:54:30Z","ordinal":2,"type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"premium","primary":null,"secondary":null,"plan_type":"free"}}}"#,
            r#"{"timestamp":"2026-08-14T14:32:02Z","ordinal":3,"type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":99.0,"window_minutes":43200,"resets_at":1789293712},"secondary":null,"credits":{"has_credits":false},"plan_type":"free"}}}"#,
        ]
        .join("\n")
    }

    #[test]
    fn reads_the_newest_non_empty_rate_limits() {
        let dir = std::env::temp_dir().join(format!("ai-hud-codex-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rollout-test.jsonl");
        std::fs::write(&path, sample_rollout()).unwrap();

        let (ts, limits) = rate_limits_in(&path).expect("rate limits found");
        assert_eq!(ts, "2026-08-14T14:32:02Z");
        assert_eq!(limits.plan_type.as_deref(), Some("free"));

        let primary = limits.primary.expect("primary window");
        let window: UsageWindow = primary.into();
        assert_eq!(window.utilization_pct, 99.0);
        // The whole point: a 43200-minute window is monthly, not 5-hourly.
        assert_eq!(window.window_minutes, Some(43_200));
        assert_eq!(window.label.as_deref(), Some("Aylık"));
        assert!(limits.secondary.is_none(), "free plan has no weekly window");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_with_only_empty_rate_limits_yields_nothing() {
        let dir = std::env::temp_dir().join(format!("ai-hud-codex-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rollout-empty.jsonl");
        std::fs::write(
            &path,
            r#"{"timestamp":"2026-08-14T13:54:30Z","type":"event_msg","payload":{"type":"token_count","rate_limits":{"primary":null,"secondary":null}}}"#,
        )
        .unwrap();

        assert!(rate_limits_in(&path).is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The endpoint spells the duration differently; it must land on the same
    /// label as the rollout log's `window_minutes`.
    #[test]
    fn endpoint_window_seconds_produce_the_same_label() {
        let raw: RawWindow = serde_json::from_str(
            r#"{"used_percent":99.0,"reset_at":1789293712,"limit_window_seconds":2592000}"#,
        )
        .unwrap();
        let window: UsageWindow = raw.into();
        assert_eq!(window.window_minutes, Some(43_200));
        assert_eq!(window.label.as_deref(), Some("Aylık"));
    }

    /// Older responses without the duration field must still parse, just
    /// without a label (the frontend then falls back to the slot name).
    #[test]
    fn endpoint_without_window_seconds_still_parses() {
        let raw: RawWindow =
            serde_json::from_str(r#"{"used_percent":12.5,"reset_at":1789293712}"#).unwrap();
        let window: UsageWindow = raw.into();
        assert_eq!(window.window_minutes, None);
        assert_eq!(window.label, None);
    }
}
