//! Reads the OAuth credentials that the official `claude` and `codex` CLIs
//! already store locally after `claude login` / `codex login`. We never ask
//! the user for a token; we just read the same file the CLI itself writes.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| anyhow!("could not resolve home directory"))
}

#[derive(Debug, Deserialize)]
struct ClaudeCredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: ClaudeOAuth,
}

#[derive(Debug, Deserialize)]
struct ClaudeOAuth {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: i64,
}

pub struct ClaudeToken {
    pub access_token: String,
    pub expires_at_ms: i64,
}

/// Reads `~/.claude/.credentials.json`, written by `claude login`.
pub fn read_claude_token() -> Result<ClaudeToken> {
    let path = home_dir()?.join(".claude").join(".credentials.json");
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("no Claude Code credentials at {}", path.display()))?;
    let parsed: ClaudeCredentialsFile =
        serde_json::from_str(&raw).context("could not parse Claude Code credentials.json")?;
    Ok(ClaudeToken {
        access_token: parsed.claude_ai_oauth.access_token,
        expires_at_ms: parsed.claude_ai_oauth.expires_at,
    })
}

pub struct CodexToken {
    pub access_token: String,
    pub account_id: Option<String>,
}

/// Reads `~/.codex/auth.json`, written by `codex login`.
///
/// NOTE: unlike the Claude credentials file, this schema is not confirmed
/// against a real file yet (no Codex account was available while scaffolding
/// this project). We parse defensively via `serde_json::Value` and try a
/// handful of plausible key paths so this degrades to a clear error instead
/// of a panic if the real shape differs. Verify against an actual
/// `~/.codex/auth.json` and tighten this once confirmed.
pub fn read_codex_token() -> Result<CodexToken> {
    let path = home_dir()?.join(".codex").join("auth.json");
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("no Codex credentials at {}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).context("could not parse Codex auth.json")?;

    let access_token = value
        .pointer("/tokens/access_token")
        .or_else(|| value.pointer("/access_token"))
        .or_else(|| value.pointer("/OPENAI_API_KEY"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("could not find an access token in ~/.codex/auth.json (schema unconfirmed)"))?
        .to_string();

    let account_id = value
        .pointer("/tokens/account_id")
        .or_else(|| value.pointer("/account_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(CodexToken {
        access_token,
        account_id,
    })
}
