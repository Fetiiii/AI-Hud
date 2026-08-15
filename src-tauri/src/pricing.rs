//! Per-model token prices.
//!
//! Prices are not hardcoded: they move, and a stale table silently produces
//! wrong money. They come from <https://models.dev/api.json>, a public,
//! keyless catalogue that reports USD per *million* tokens broken out into
//! input / output / cache-read / cache-write - which is the breakdown this
//! app needs, because a cached read costs roughly a tenth of a fresh input
//! token and most of a coding session's tokens are cached reads.
//!
//! Order of precedence:
//!   1. `~/.config/ai-hud/prices.json` - anything the user sets wins.
//!   2. `~/.cache/ai-hud/prices.json`  - last successful fetch.
//!   3. A live fetch, which then refreshes the cache.
//!
//! With no network and no cache there are simply no prices, and the UI says
//! so rather than inventing a number.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const CATALOGUE_URL: &str = "https://models.dev/api.json";

/// How long a cached catalogue stays fresh. Prices change on the order of
/// months; a day is plenty and keeps startup off the network most of the time.
const CACHE_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// USD per million tokens.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ModelRates {
    pub input: f64,
    pub output: f64,
    #[serde(default)]
    pub cache_read: f64,
    /// Writing to the cache. Anthropic charges this for the 5-minute TTL;
    /// the 1-hour TTL is dearer - see [`ModelRates::cache_write_1h`].
    #[serde(default)]
    pub cache_write: f64,
}

impl ModelRates {
    /// Anthropic prices a 1-hour cache write at 2x the input rate, against
    /// 1.25x for the 5-minute one. The catalogue only carries the 5-minute
    /// figure, and the difference is not small: a long coding session writes
    /// a lot of 1-hour cache, so charging it at the 5-minute rate would
    /// under-report the bill.
    pub fn cache_write_1h(&self) -> f64 {
        self.input * 2.0
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriceTable {
    /// Keyed by the model id exactly as the provider writes it into its logs.
    pub models: HashMap<String, ModelRates>,
    pub fetched_at_ms: u64,
    pub source: String,
}

impl PriceTable {
    pub fn rates_for(&self, model: &str) -> Option<ModelRates> {
        if let Some(r) = self.models.get(model) {
            return Some(*r);
        }
        // Providers sometimes append a dated suffix to the id they log
        // (`claude-haiku-4-5-20251001`); fall back to the longest catalogue
        // entry that the logged id starts with.
        self.models
            .iter()
            .filter(|(id, _)| model.starts_with(id.as_str()))
            .max_by_key(|(id, _)| id.len())
            .map(|(_, r)| *r)
    }

    fn is_fresh(&self, now_ms: u64) -> bool {
        !self.models.is_empty() && now_ms.saturating_sub(self.fetched_at_ms) < CACHE_TTL_MS
    }
}

// --- models.dev response shape -------------------------------------------

#[derive(Debug, Deserialize)]
struct RawCost {
    #[serde(default)]
    input: Option<f64>,
    #[serde(default)]
    output: Option<f64>,
    #[serde(default)]
    cache_read: Option<f64>,
    #[serde(default)]
    cache_write: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawModel {
    #[serde(default)]
    cost: Option<RawCost>,
}

#[derive(Debug, Deserialize)]
struct RawProvider {
    #[serde(default)]
    models: HashMap<String, RawModel>,
}

fn parse_catalogue(body: &str) -> Result<HashMap<String, ModelRates>> {
    let raw: HashMap<String, RawProvider> =
        serde_json::from_str(body).context("models.dev catalogue was not the expected shape")?;

    let mut models = HashMap::new();
    for provider in raw.values() {
        for (id, model) in &provider.models {
            let Some(cost) = &model.cost else { continue };
            let (Some(input), Some(output)) = (cost.input, cost.output) else {
                continue;
            };
            models.insert(
                id.clone(),
                ModelRates {
                    input,
                    output,
                    // A model with no cache pricing is charged at the plain
                    // input rate for reads rather than treated as free.
                    cache_read: cost.cache_read.unwrap_or(input),
                    cache_write: cost.cache_write.unwrap_or(input),
                },
            );
        }
    }
    Ok(models)
}

// --- on-disk locations ----------------------------------------------------

fn cache_path() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("ai-hud").join("prices.json"))
}

pub fn override_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("ai-hud").join("prices.json"))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Option<T> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn apply_overrides(table: &mut PriceTable) {
    let Some(path) = override_path() else { return };
    let Some(user): Option<HashMap<String, ModelRates>> = read_json(&path) else {
        return;
    };
    for (model, rates) in user {
        table.models.insert(model, rates);
    }
    table.source = format!("{} + {}", table.source, path.display());
}

/// Best available price table. Never fails outright: a network error just
/// means falling back to the cache, and no cache means an empty table.
pub async fn load(client: &reqwest::Client, now_ms: u64) -> (PriceTable, Option<String>) {
    let cached: Option<PriceTable> = cache_path().and_then(|p| read_json(&p));

    if let Some(table) = cached.as_ref().filter(|t| t.is_fresh(now_ms)) {
        let mut table = table.clone();
        apply_overrides(&mut table);
        return (table, None);
    }

    match fetch(client, now_ms).await {
        Ok(mut table) => {
            if let Some(path) = cache_path() {
                let _ = std::fs::create_dir_all(path.parent().unwrap());
                if let Ok(json) = serde_json::to_string(&table) {
                    let _ = std::fs::write(&path, json);
                }
            }
            apply_overrides(&mut table);
            (table, None)
        }
        Err(e) => {
            let note = format!("fiyat listesi güncellenemedi: {e:#}");
            match cached {
                Some(mut stale) => {
                    apply_overrides(&mut stale);
                    (stale, Some(format!("{note} (önbellekteki liste kullanılıyor)")))
                }
                None => (PriceTable::default(), Some(note)),
            }
        }
    }
}

async fn fetch(client: &reqwest::Client, now_ms: u64) -> Result<PriceTable> {
    let body = client
        .get(CATALOGUE_URL)
        .header("user-agent", "ai-hud/0.1.0")
        .send()
        .await
        .context("could not reach models.dev")?
        .error_for_status()
        .context("models.dev returned an error status")?
        .text()
        .await
        .context("could not read the models.dev response")?;

    Ok(PriceTable {
        models: parse_catalogue(&body)?,
        fetched_at_ms: now_ms,
        source: CATALOGUE_URL.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_catalogue_shape() {
        let body = r#"{
          "anthropic": {"models": {
            "claude-opus-5": {"cost": {"input": 5, "output": 25, "cache_read": 0.5, "cache_write": 6.25}},
            "claude-haiku-4-5": {"cost": {"input": 1, "output": 5}}
          }},
          "openai": {"models": {
            "gpt-5.6-terra": {"cost": {"input": 2, "output": 12, "cache_read": 0.2, "cache_write": 2.5}}
          }}
        }"#;
        let models = parse_catalogue(body).unwrap();
        assert_eq!(models.len(), 3);

        let opus = models["claude-opus-5"];
        assert_eq!(opus.input, 5.0);
        assert_eq!(opus.cache_read, 0.5);
        // A 1-hour cache write is 2x input, not the catalogue's 5-minute figure.
        assert_eq!(opus.cache_write, 6.25);
        assert_eq!(opus.cache_write_1h(), 10.0);

        // Missing cache pricing falls back to the input rate, never to free.
        assert_eq!(models["claude-haiku-4-5"].cache_read, 1.0);
    }

    #[test]
    fn matches_dated_model_ids_by_longest_prefix() {
        let mut models = HashMap::new();
        models.insert("claude-haiku-4-5".to_string(), ModelRates { input: 1.0, ..Default::default() });
        models.insert("claude-haiku".to_string(), ModelRates { input: 99.0, ..Default::default() });
        let table = PriceTable { models, ..Default::default() };

        assert_eq!(table.rates_for("claude-haiku-4-5").unwrap().input, 1.0);
        // The dated id logged by the CLI still resolves, and to the *longest*
        // matching entry rather than the vaguest one.
        assert_eq!(table.rates_for("claude-haiku-4-5-20251001").unwrap().input, 1.0);
        assert!(table.rates_for("gpt-5.6").is_none());
    }
}
