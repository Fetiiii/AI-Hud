//! Reads the OAuth credentials that the official `claude` and `codex` CLIs
//! already store locally after `claude login` / `codex login`. We never ask
//! the user for a token; we just read the same file the CLI itself writes.

use anyhow::{anyhow, bail, Context, Result};
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

#[derive(Debug, Deserialize)]
struct CodexAuthFile {
    tokens: Option<CodexTokens>,
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexTokens {
    access_token: String,
    account_id: Option<String>,
}

/// Reads `~/.codex/auth.json`, written by `codex login`. Schema confirmed
/// against a real file: `{ auth_mode, OPENAI_API_KEY, tokens: { id_token,
/// access_token, refresh_token, account_id }, last_refresh }`.
pub fn read_codex_token() -> Result<CodexToken> {
    let path = home_dir()?.join(".codex").join("auth.json");
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("no Codex credentials at {}", path.display()))?;
    let parsed: CodexAuthFile =
        serde_json::from_str(&raw).context("could not parse Codex auth.json")?;

    if let Some(tokens) = parsed.tokens {
        return Ok(CodexToken {
            access_token: tokens.access_token,
            account_id: tokens.account_id,
        });
    }
    if let Some(access_token) = parsed.openai_api_key {
        return Ok(CodexToken {
            access_token,
            account_id: None,
        });
    }
    bail!("~/.codex/auth.json has neither `tokens` nor `OPENAI_API_KEY` - run `codex` to log in")
}
