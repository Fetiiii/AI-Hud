//! "Active session context usage" - how full the current conversation's
//! context window is. Unlike the 5h/weekly limits this needs no network
//! call: both Claude Code and Codex write every turn's token usage straight
//! into local transcript/rollout files, so we just read the most recently
//! touched one per provider.

use anyhow::{anyhow, Result};
use serde::Serialize;
use std::io::{BufRead, BufReader};
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

/// Fallback when the transcript names no model we recognise. 200k is the
/// smaller of the two window sizes in use, so an unknown model errs towards
/// reporting a *fuller* context rather than a falsely comfortable one.
const DEFAULT_CLAUDE_CONTEXT_WINDOW: u64 = 200_000;

/// Context window for a Claude model, by name as it appears in the transcript.
///
/// This matters more than it looks: assuming 200k for everything reported
/// "%269 - 537.7K / 200.0K" for a Sonnet 5 session, because that model
/// actually has a 1M window. The current Opus/Sonnet/Fable line is all 1M;
/// Haiku is the 200k exception.
fn claude_context_window(model: &str) -> u64 {
    if model.contains("haiku") {
        return 200_000;
    }
    if model.contains("opus")
        || model.contains("sonnet")
        || model.contains("fable")
        || model.contains("mythos")
    {
        return 1_000_000;
    }
    DEFAULT_CLAUDE_CONTEXT_WINDOW
}

/// Fallback only - Codex's `token_count` events carry the real
/// `model_context_window` almost every time, so this rarely matters.
const DEFAULT_CODEX_CONTEXT_WINDOW: u64 = 128_000;

pub fn find_latest_file(root: &Path, ext: &str) -> Option<PathBuf> {
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
fn last_claude_usage(path: &Path) -> Result<Option<(u64, u64)>> {
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
        // The model is recorded on the same entry as the usage it produced,
        // so a session that switched models mid-way is sized by whichever
        // model actually ran the last turn.
        let window = value
            .pointer("/message/model")
            .and_then(|v| v.as_str())
            .map(claude_context_window)
            .unwrap_or(DEFAULT_CLAUDE_CONTEXT_WINDOW);
        return Ok(Some((input + cache_creation + cache_read, window)));
    }
    Ok(None)
}

pub fn claude_context_usage() -> Result<Option<ContextUsage>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("no home dir"))?;
    let projects_dir = home.join(".claude").join("projects");
    let Some(path) = find_latest_file(&projects_dir, "jsonl") else {
        return Ok(None);
    };
    let Some((tokens, context_window)) = last_claude_usage(&path)? else {
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
        context_window,
        project,
    }))
}

/// Pulls token usage out of a Codex rollout log
/// (`~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`). Each line is
/// `{ timestamp, ordinal, type, payload }`; the ones we want have
/// `type == "event_msg"` and `payload.type == "token_count"`, carrying
/// `payload.info.last_token_usage` (this turn's tokens) and
/// `payload.info.model_context_window`.
///
/// We total `input_tokens + cached_input_tokens + cache_write_input_tokens`
/// from the *last* such event, mirroring the Claude-side formula: it's what
/// was actually sent to the model on the most recent turn, i.e. how full the
/// context window is right now.
fn last_codex_usage(path: &Path) -> Result<Option<(u64, u64)>> {
    let raw = std::fs::read_to_string(path)?;
    for line in raw.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("type").and_then(|v| v.as_str()) != Some("event_msg") {
            continue;
        }
        let payload = value.get("payload");
        if payload.and_then(|p| p.get("type")).and_then(|v| v.as_str()) != Some("token_count") {
            continue;
        }
        let Some(info) = payload.and_then(|p| p.get("info")) else {
            continue;
        };
        let Some(usage) = info.get("last_token_usage") else {
            continue;
        };

        let field = |name: &str| usage.get(name).and_then(|v| v.as_u64()).unwrap_or(0);
        let tokens = field("input_tokens") + field("cached_input_tokens") + field("cache_write_input_tokens");
        let window = info
            .get("model_context_window")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_CODEX_CONTEXT_WINDOW);

        return Ok(Some((tokens, window)));
    }
    Ok(None)
}

/// The rollout's first line is always `session_meta`, carrying `payload.cwd`
/// - cheap to read without scanning the whole (possibly large) file.
fn codex_session_cwd(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut first_line = String::new();
    BufReader::new(file).read_line(&mut first_line).ok()?;
    let value: serde_json::Value = serde_json::from_str(first_line.trim()).ok()?;
    value
        .pointer("/payload/cwd")
        .and_then(|v| v.as_str())
        .map(|cwd| {
            Path::new(cwd)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(cwd)
                .to_string()
        })
}

pub fn codex_context_usage() -> Result<Option<ContextUsage>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("no home dir"))?;
    let sessions_dir = home.join(".codex").join("sessions");
    let Some(path) = find_latest_file(&sessions_dir, "jsonl") else {
        return Ok(None);
    };
    let Some((tokens, context_window)) = last_codex_usage(&path)? else {
        return Ok(None);
    };

    Ok(Some(ContextUsage {
        provider: "codex",
        tokens,
        context_window,
        project: codex_session_cwd(&path),
    }))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Shape as described by a real `~/.codex/sessions/**/rollout-*.jsonl`:
    /// `session_meta` first, then arbitrary events, with `token_count`
    /// `event_msg`s scattered throughout (only the last one should win).
    fn sample_rollout() -> String {
        [
            r#"{"timestamp":"2026-08-14T13:54:20Z","ordinal":0,"type":"session_meta","payload":{"session_id":"019fffe8-93fd-7ad2-b9f9-b6471109bc48","cwd":"/home/user/some-project","originator":"codex-tui"}}"#,
            r#"{"timestamp":"2026-08-14T13:54:25Z","ordinal":1,"type":"turn_context","payload":{"turn_id":"t1","cwd":"/home/user/some-project"}}"#,
            r#"{"timestamp":"2026-08-14T13:54:30Z","ordinal":2,"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":500,"cached_input_tokens":0,"cache_write_input_tokens":0,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":550},"last_token_usage":{"input_tokens":500,"cached_input_tokens":0,"cache_write_input_tokens":0,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":550},"model_context_window":272000},"rate_limits":{"limit_id":"x"}}}"#,
            r#"{"timestamp":"2026-08-14T13:55:00Z","ordinal":3,"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":9000,"cached_input_tokens":1200,"cache_write_input_tokens":300,"output_tokens":400,"reasoning_output_tokens":100,"total_tokens":11000},"last_token_usage":{"input_tokens":6000,"cached_input_tokens":1200,"cache_write_input_tokens":300,"output_tokens":400,"reasoning_output_tokens":100,"total_tokens":8000},"model_context_window":272000},"rate_limits":{"limit_id":"x","primary":{"used_percent":12.5,"window_minutes":300,"resets_at":"2026-08-14T18:54:20Z"}}}}"#,
        ]
        .join("\n")
    }

    #[test]
    fn picks_the_last_token_count_event() {
        let dir = std::env::temp_dir().join(format!("ai-hud-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rollout-test.jsonl");
        std::fs::write(&path, sample_rollout()).unwrap();

        let (tokens, window) = last_codex_usage(&path).unwrap().expect("usage found");
        // last_token_usage from the *second* token_count event: 6000 + 1200 + 300
        assert_eq!(tokens, 7500);
        assert_eq!(window, 272_000);

        let cwd = codex_session_cwd(&path);
        assert_eq!(cwd.as_deref(), Some("some-project"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sizes_the_window_from_the_model_that_ran_the_turn() {
        // Regression: this reported "%269 - 537.7K / 200.0K" on screen,
        // because every model was assumed to have a 200k window.
        assert_eq!(claude_context_window("claude-sonnet-5"), 1_000_000);
        assert_eq!(claude_context_window("claude-opus-5"), 1_000_000);
        assert_eq!(claude_context_window("claude-haiku-4-5-20251001"), 200_000);
        assert_eq!(claude_context_window("something-unknown"), 200_000);
    }

    #[test]
    fn reads_tokens_and_window_from_a_transcript() {
        let dir = std::env::temp_dir().join(format!("ai-hud-claude-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("transcript.jsonl");
        std::fs::write(
            &path,
            r#"{"type":"assistant","message":{"model":"claude-sonnet-5","usage":{"input_tokens":1200,"cache_creation_input_tokens":300,"cache_read_input_tokens":536200}}}"#,
        )
        .unwrap();

        let (tokens, window) = last_claude_usage(&path).unwrap().expect("usage found");
        assert_eq!(tokens, 537_700);
        assert_eq!(window, 1_000_000, "Sonnet 5 has a 1M window, not 200k");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_token_count_returns_none() {
        let dir = std::env::temp_dir().join(format!("ai-hud-test-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rollout-empty.jsonl");
        std::fs::write(
            &path,
            r#"{"timestamp":"2026-08-14T13:54:20Z","ordinal":0,"type":"session_meta","payload":{"cwd":"/home/user/x"}}"#,
        )
        .unwrap();

        assert!(last_codex_usage(&path).unwrap().is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
