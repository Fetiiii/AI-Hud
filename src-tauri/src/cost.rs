//! What the active session would have cost at API list prices.
//!
//! Two things make this less trivial than summing a column.
//!
//! **Duplicates.** Claude Code writes the same assistant message into its
//! transcript more than once. In a real session here, 628 usage-bearing lines
//! carried only 361 distinct `message.id`s - summing them raw inflated the
//! token total by 73%. Entries are keyed on `(message.id, requestId)` and
//! counted once.
//!
//! **Cache tiers.** Most tokens in a coding session are cache *reads*, priced
//! at roughly a tenth of a fresh input token, and cache *writes* are priced by
//! how long the entry lives (Anthropic: 1.25x input for the 5-minute TTL, 2x
//! for the 1-hour one). Anthropic reports the split under `cache_creation`,
//! so the two are charged separately rather than at one blended rate.
//!
//! The figure is an *estimate at list prices*: these are subscription
//! products, so it answers "what would this have cost through the API", not
//! "what you were billed".

use crate::pricing::PriceTable;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenTotals {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    /// 5-minute-TTL cache writes.
    pub cache_write_5m: u64,
    /// 1-hour-TTL cache writes, which bill higher.
    pub cache_write_1h: u64,
}

impl TokenTotals {
    fn add(&mut self, other: &TokenTotals) {
        self.input += other.input;
        self.output += other.output;
        self.cache_read += other.cache_read;
        self.cache_write_5m += other.cache_write_5m;
        self.cache_write_1h += other.cache_write_1h;
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCost {
    pub model: String,
    pub tokens: TokenTotals,
    /// `None` when the model isn't in the price table - shown as unpriced
    /// rather than silently counted as free.
    pub usd: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCost {
    pub tokens: TokenTotals,
    /// Sum over the models that could be priced.
    pub usd: f64,
    /// True when at least one model had no price, so `usd` is a lower bound.
    pub partial: bool,
    pub models: Vec<ModelCost>,
}

fn usd_for(rates: &crate::pricing::ModelRates, t: &TokenTotals) -> f64 {
    let per_million = |tokens: u64, rate: f64| (tokens as f64) * rate / 1_000_000.0;
    per_million(t.input, rates.input)
        + per_million(t.output, rates.output)
        + per_million(t.cache_read, rates.cache_read)
        + per_million(t.cache_write_5m, rates.cache_write)
        + per_million(t.cache_write_1h, rates.cache_write_1h())
}

fn summarise(by_model: HashMap<String, TokenTotals>, prices: &PriceTable) -> SessionCost {
    let mut out = SessionCost::default();
    let mut models: Vec<ModelCost> = by_model
        .into_iter()
        .map(|(model, tokens)| {
            let usd = prices.rates_for(&model).map(|r| usd_for(&r, &tokens));
            out.tokens.add(&tokens);
            match usd {
                Some(v) => out.usd += v,
                None => out.partial = true,
            }
            ModelCost { model, tokens, usd }
        })
        .collect();
    // Dearest first: that is the line a reader is looking for.
    models.sort_by(|a, b| {
        b.usd
            .unwrap_or(0.0)
            .partial_cmp(&a.usd.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.models = models;
    out
}

/// Walks a Claude Code transcript and totals its usage, per model.
pub fn claude_session_cost(path: &Path, prices: &PriceTable) -> Option<SessionCost> {
    let raw = std::fs::read_to_string(path).ok()?;
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut by_model: HashMap<String, TokenTotals> = HashMap::new();

    for line in raw.lines() {
        if !line.contains("\"usage\"") {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(message) = value.get("message") else {
            continue;
        };
        let Some(usage) = message.get("usage") else {
            continue;
        };

        // The same assistant message appears more than once; count it once.
        let key = (
            message.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            value.get("requestId").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        );
        if !key.0.is_empty() && !seen.insert(key) {
            continue;
        }

        let field = |name: &str| usage.get(name).and_then(|v| v.as_u64()).unwrap_or(0);
        let creation = usage.get("cache_creation");
        let tier = |name: &str| {
            creation
                .and_then(|c| c.get(name))
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        };

        let write_5m = tier("ephemeral_5m_input_tokens");
        let write_1h = tier("ephemeral_1h_input_tokens");
        let write_total = field("cache_creation_input_tokens");

        let totals = TokenTotals {
            input: field("input_tokens"),
            // `output_tokens` already includes thinking tokens; the
            // `output_tokens_details.thinking_tokens` field is a breakdown of
            // it, not an addition to it.
            output: field("output_tokens"),
            cache_read: field("cache_read_input_tokens"),
            // Older entries carry no per-TTL split. Attribute those to the
            // 5-minute tier, which is the cheaper of the two, so an unknown
            // never inflates the estimate.
            cache_write_5m: if write_5m + write_1h == 0 {
                write_total
            } else {
                write_5m
            },
            cache_write_1h: write_1h,
        };

        let model = message
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("bilinmeyen")
            .to_string();
        by_model.entry(model).or_default().add(&totals);
    }

    if by_model.is_empty() {
        return None;
    }
    Some(summarise(by_model, prices))
}

/// Codex rollout logs already carry a running `total_token_usage`, so the
/// last one wins rather than being summed - adding them would count every
/// turn's cumulative figure again.
pub fn codex_session_cost(path: &Path, prices: &PriceTable) -> Option<SessionCost> {
    let raw = std::fs::read_to_string(path).ok()?;
    let mut model = "bilinmeyen".to_string();
    let mut latest: Option<TokenTotals> = None;

    for line in raw.lines() {
        if line.contains("\"model\"") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(m) = v.pointer("/payload/model").and_then(|v| v.as_str()) {
                    model = m.to_string();
                }
            }
        }
        if !line.contains("\"total_token_usage\"") {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(total) = value.pointer("/payload/info/total_token_usage") else {
            continue;
        };
        let field = |name: &str| total.get(name).and_then(|v| v.as_u64()).unwrap_or(0);
        latest = Some(TokenTotals {
            input: field("input_tokens"),
            output: field("output_tokens"),
            cache_read: field("cached_input_tokens"),
            cache_write_5m: field("cache_write_input_tokens"),
            cache_write_1h: 0,
        });
    }

    let totals = latest?;
    let mut by_model = HashMap::new();
    by_model.insert(model, totals);
    Some(summarise(by_model, prices))
}

/// Cost of the session each provider most recently wrote to - the same files
/// the context readout already tracks, so the two always describe the same
/// conversation.
pub fn session_costs(prices: &PriceTable) -> (Option<SessionCost>, Option<SessionCost>) {
    let Some(home) = dirs::home_dir() else {
        return (None, None);
    };

    let claude = crate::context::find_latest_file(&home.join(".claude").join("projects"), "jsonl")
        .and_then(|p| claude_session_cost(&p, prices));
    let codex = crate::context::find_latest_file(&home.join(".codex").join("sessions"), "jsonl")
        .and_then(|p| codex_session_cost(&p, prices));

    (claude, codex)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::ModelRates;

    fn prices() -> PriceTable {
        let mut models = HashMap::new();
        models.insert(
            "claude-sonnet-5".to_string(),
            ModelRates { input: 2.0, output: 10.0, cache_read: 0.2, cache_write: 2.5 },
        );
        PriceTable { models, ..Default::default() }
    }

    fn write(name: &str, body: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ai-hud-cost-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.jsonl");
        std::fs::write(&path, body).unwrap();
        path
    }

    /// The duplicate-entry problem, in miniature: the same message twice.
    #[test]
    fn counts_a_repeated_message_once() {
        let entry = r#"{"requestId":"req_1","message":{"id":"msg_1","model":"claude-sonnet-5","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":1000,"cache_creation_input_tokens":0}}}"#;
        let path = write("dup", &format!("{entry}\n{entry}\n"));

        let cost = claude_session_cost(&path, &prices()).unwrap();
        assert_eq!(cost.tokens.input, 100, "the repeat must not be added again");
        assert_eq!(cost.tokens.output, 50);
        assert_eq!(cost.tokens.cache_read, 1000);

        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn charges_each_cache_tier_at_its_own_rate() {
        let path = write(
            "tiers",
            r#"{"requestId":"r","message":{"id":"m","model":"claude-sonnet-5","usage":{"input_tokens":0,"output_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":2000000,"cache_creation":{"ephemeral_5m_input_tokens":1000000,"ephemeral_1h_input_tokens":1000000}}}}"#,
        );

        let cost = claude_session_cost(&path, &prices()).unwrap();
        assert_eq!(cost.tokens.cache_write_5m, 1_000_000);
        assert_eq!(cost.tokens.cache_write_1h, 1_000_000);
        // 1M at the 5-minute rate ($2.50) + 1M at the 1-hour rate (2x input = $4.00).
        assert!((cost.usd - 6.5).abs() < 1e-9, "got {}", cost.usd);

        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// A cache read must not be billed as a fresh input token - that is the
    /// mistake that would overstate a coding session several times over.
    #[test]
    fn prices_cache_reads_far_below_input() {
        let path = write(
            "reads",
            r#"{"requestId":"r","message":{"id":"m","model":"claude-sonnet-5","usage":{"input_tokens":0,"output_tokens":0,"cache_read_input_tokens":1000000,"cache_creation_input_tokens":0}}}"#,
        );

        let cost = claude_session_cost(&path, &prices()).unwrap();
        assert!((cost.usd - 0.20).abs() < 1e-9, "1M cache reads = $0.20, got {}", cost.usd);

        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn flags_a_model_it_cannot_price() {
        let path = write(
            "unpriced",
            r#"{"requestId":"r","message":{"id":"m","model":"some-new-model","usage":{"input_tokens":1000,"output_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}"#,
        );

        let cost = claude_session_cost(&path, &prices()).unwrap();
        assert!(cost.partial, "an unpriced model makes the total a lower bound");
        assert_eq!(cost.usd, 0.0);
        assert!(cost.models[0].usd.is_none());

        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// Codex reports a running total, so the last line is the answer.
    #[test]
    fn codex_takes_the_latest_running_total() {
        let path = write(
            "codex",
            &[
                r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"output_tokens":10,"cached_input_tokens":0,"cache_write_input_tokens":0}}}}"#,
                r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":500,"output_tokens":60,"cached_input_tokens":20,"cache_write_input_tokens":5}}}}"#,
            ]
            .join("\n"),
        );

        let cost = codex_session_cost(&path, &prices()).unwrap();
        assert_eq!(cost.tokens.input, 500, "running totals must not be summed");
        assert_eq!(cost.tokens.output, 60);

        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
