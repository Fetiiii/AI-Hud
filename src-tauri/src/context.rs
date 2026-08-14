//! "Active session context usage" - how full the current conversation's
//! context window is. Unlike the 5h/weekly limits this needs no network
//! call: Claude Code writes every message's token usage straight into its
//! local transcript files, so we just read the most recently touched one.

use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextUsage {
    pub provider: &'static str,
    pub tokens: u64,
    pub context_window: u64,
    pub project: Option<String>,
}

/// Standard Claude context window. A handful of models/betas support 1M,
/// but 200k is the safe default until we read the model name and special-case it.
const DEFAULT_CLAUDE_CONTEXT_WINDOW: u64 = 200_000;

fn find_latest_file(root: &Path, ext: &str) -> Option<PathBuf> {
    fn walk(dir: &Path, ext: &str, best: &mut Option<(SystemTime, PathBuf)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, ext, best);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some(ext) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            if best.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
                *best = Some((modified, path));
            }
        }
    }

    let mut best = None;
    walk(root, ext, &mut best);
    best.map(|(_, path)| path)
}

/// Pulls the token usage of the last assistant turn out of a Claude Code
/// transcript (`.jsonl`, one JSON object per line). We total
/// `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`
/// from the *last* usage block, which is the closest local proxy for "how
/// much of the context window the next turn will send".
fn last_claude_usage(path: &Path) -> Result<Option<u64>> {
    let raw = std::fs::read_to_string(path)?;
    for line in raw.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(usage) = value.pointer("/message/usage") else {
            continue;
        };
        let input = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cache_creation = usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let cache_read = usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        return Ok(Some(input + cache_creation + cache_read));
    }
    Ok(None)
}

pub fn claude_context_usage() -> Result<Option<ContextUsage>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("no home dir"))?;
    let projects_dir = home.join(".claude").join("projects");
    let Some(path) = find_latest_file(&projects_dir, "jsonl") else {
        return Ok(None);
    };
    let Some(tokens) = last_claude_usage(&path)? else {
        return Ok(None);
    };
    let project = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    Ok(Some(ContextUsage {
        provider: "claude",
        tokens,
        context_window: DEFAULT_CLAUDE_CONTEXT_WINDOW,
        project,
    }))
}

/// UNVERIFIED: Codex CLI's local session log format/location was not
/// confirmed against a real installation while scaffolding this project.
/// Wire this up the same way as `claude_context_usage` once `~/.codex`'s
/// actual log layout is confirmed.
pub fn codex_context_usage() -> Result<Option<ContextUsage>> {
    Ok(None)
}

/// Directories worth watching for live updates (used by the file-watcher in
/// `lib.rs`). Missing directories are fine - `notify` just won't fire for them.
pub fn watch_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".claude").join("projects"));
        roots.push(home.join(".codex").join("sessions"));
    }
    roots
}
