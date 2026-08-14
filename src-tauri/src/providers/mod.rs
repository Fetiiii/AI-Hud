pub mod claude;
pub mod codex;

use serde::Serialize;

/// One rate-limit window (e.g. "5 hour" or "7 day").
///
/// `resets_at` is passed through as the raw ISO-8601/RFC-3339 string from
/// the provider; we deliberately do *not* parse it in Rust so the frontend
/// can do the countdown math in the user's local timezone with plain `Date`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub utilization_pct: f64,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
    pub seven_day_opus: Option<UsageWindow>,
    pub seven_day_sonnet: Option<UsageWindow>,
}
