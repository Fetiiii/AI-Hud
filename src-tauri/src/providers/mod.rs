pub mod claude;
pub mod codex;

use serde::Serialize;

/// One rate-limit window (e.g. "5 hour" or "7 day").
///
/// `resets_at` is passed through as the raw ISO-8601/RFC-3339 string from
/// the provider; we deliberately do *not* parse it in Rust so the frontend
/// can do the countdown math in the user's local timezone with plain `Date`.
///
/// `label` carries the window's *actual* duration when the provider tells us
/// (Codex does, via `window_minutes` / `limit_window_seconds`). Assuming a
/// fixed meaning per slot is wrong: on a free ChatGPT plan Codex's primary
/// window is monthly, not 5-hourly, so a hardcoded "5 saatlik" label reads
/// as nonsense next to a 29-day reset. `None` means "no duration reported"
/// and the frontend falls back to the slot's conventional name (Claude's
/// endpoint names its windows structurally and needs no override).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub utilization_pct: f64,
    pub resets_at: Option<String>,
    pub label: Option<String>,
    pub window_minutes: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
    pub seven_day_opus: Option<UsageWindow>,
    pub seven_day_sonnet: Option<UsageWindow>,
    /// `free` / `plus` / `pro` where the provider reports it (Codex does, in
    /// its rollout logs). Shown in the detail view.
    pub plan_type: Option<String>,
}

/// Turns a window duration into the name a user would recognise. The three
/// exact matches are the windows the providers actually issue; anything else
/// gets a generic rendering rather than being forced into one of them.
pub fn window_label(minutes: u64) -> String {
    match minutes {
        300 => "5 saatlik".to_string(),
        10_080 => "Haftalık".to_string(),
        43_200 => "Aylık".to_string(),
        m if m < 60 => format!("{m} dakikalık"),
        m if m < 1440 => format!("{} saatlik", m / 60),
        m => format!("{} günlük", m / 1440),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_the_windows_providers_actually_issue() {
        assert_eq!(window_label(300), "5 saatlik");
        assert_eq!(window_label(10_080), "Haftalık");
        // The free-plan Codex window that was being mislabelled "5 saatlik".
        assert_eq!(window_label(43_200), "Aylık");
    }

    #[test]
    fn falls_back_to_a_generic_rendering() {
        assert_eq!(window_label(30), "30 dakikalık");
        assert_eq!(window_label(180), "3 saatlik");
        assert_eq!(window_label(4320), "3 günlük");
    }
}
