use crate::context::ContextUsage;
use crate::providers::ProviderUsage;
use serde::Serialize;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub claude_usage: Option<ProviderUsage>,
    pub codex_usage: Option<ProviderUsage>,
    pub claude_context: Option<ContextUsage>,
    pub codex_context: Option<ContextUsage>,
    /// Human-readable errors from the last refresh attempt (missing login,
    /// endpoint schema drift, etc.) surfaced as-is in the detail view rather
    /// than swallowed.
    pub errors: Vec<String>,
    pub updated_at_ms: u64,
}

pub struct AppState {
    pub snapshot: Mutex<Snapshot>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            snapshot: Mutex::new(Snapshot::default()),
        }
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
