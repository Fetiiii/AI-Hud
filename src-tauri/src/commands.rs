use crate::context::{claude_context_usage, codex_context_usage};
use crate::cost::session_costs;
use crate::pricing;
use crate::providers::{claude, codex};
use crate::state::{now_ms, AppState, Snapshot};
use tauri::{AppHandle, Emitter, Manager, State};

pub const SNAPSHOT_EVENT: &str = "snapshot-updated";

/// Full refresh: hits both providers' network endpoints for the 5h/weekly
/// windows *and* re-reads local transcripts for context usage. Called on
/// startup and on a slow timer (network calls only, kept infrequent).
pub async fn refresh_all(app: &AppHandle) {
    let client = reqwest::Client::new();
    let mut errors = Vec::new();

    // Last known good, so a transient failure (a rate limit, a dropped
    // connection) leaves the card showing slightly stale numbers with a note
    // rather than blanking it out - which reads as "you are not logged in".
    let previous = app.state::<AppState>().snapshot.lock().unwrap().clone();

    let claude_usage = match claude::fetch(&client).await {
        Ok(u) => Some(u),
        Err(e) => {
            errors.push(format!("Claude: {e:#}"));
            previous.claude_usage
        }
    };
    let codex_usage = match codex::fetch(&client).await {
        Ok(u) => Some(u),
        Err(e) => {
            errors.push(format!("Codex: {e:#}"));
            previous.codex_usage
        }
    };
    let claude_context = claude_context_usage().unwrap_or_else(|e| {
        errors.push(format!("Claude context: {e:#}"));
        None
    });
    let codex_context = codex_context_usage().unwrap_or_else(|e| {
        errors.push(format!("Codex context: {e:#}"));
        None
    });

    // Prices drive the cost readout; a failure here only costs the money
    // figure, so it is a note rather than an error.
    let (prices, price_note) = pricing::load(&client, now_ms()).await;
    if let Some(note) = price_note {
        errors.push(format!("Fiyatlar: {note}"));
    }
    let (claude_cost, codex_cost) = session_costs(&prices);

    let snapshot = Snapshot {
        claude_usage,
        codex_usage,
        claude_context,
        codex_context,
        claude_cost,
        codex_cost,
        errors,
        updated_at_ms: now_ms(),
    };

    let state = app.state::<AppState>();
    *state.snapshot.lock().unwrap() = snapshot.clone();
    let _ = app.emit(SNAPSHOT_EVENT, snapshot);
}

/// Cheap refresh: only re-reads local transcripts. Triggered by the
/// filesystem watcher so context usage feels live without hammering the
/// network endpoints on every message.
pub async fn refresh_context_only(app: &AppHandle) {
    let claude_context = claude_context_usage().ok().flatten();
    let codex_context = codex_context_usage().ok().flatten();

    let state = app.state::<AppState>();
    let snapshot = {
        let mut snap = state.snapshot.lock().unwrap();
        snap.claude_context = claude_context;
        snap.codex_context = codex_context;
        snap.updated_at_ms = now_ms();
        snap.clone()
    };
    let _ = app.emit(SNAPSHOT_EVENT, snapshot);
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Snapshot {
    state.snapshot.lock().unwrap().clone()
}

#[tauri::command]
pub async fn refresh_now(app: AppHandle) -> Snapshot {
    refresh_all(&app).await;
    app.state::<AppState>().snapshot.lock().unwrap().clone()
}

#[tauri::command]
pub fn open_detail(app: AppHandle) {
    if let Some(win) = app.get_webview_window("detail") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Temporary diagnostic channel: the webview's own console is not visible
/// from the terminal, so the frontend routes findings through here.
#[tauri::command]
pub fn debug_log(msg: String) {
    eprintln!("[hud-debug] {msg}");
}

#[tauri::command]
pub fn hide_popover(app: AppHandle) {
    if let Some(win) = app.get_webview_window("popover") {
        let _ = win.hide();
    }
}
