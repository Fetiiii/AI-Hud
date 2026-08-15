mod commands;
mod context;
mod cost;
mod credentials;
mod pricing;
mod providers;
mod state;

use state::AppState;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

/// Show the HUD if it is hidden, hide it if it is showing. Deliberately does
/// not re-centre: the window reappears where the user last put it.
fn toggle_hud(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("popover") else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
    } else {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::refresh_now,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let show_item = MenuItem::with_id(app, "show", "Göster", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "quit" => app.exit(0),
                        "show" => toggle_hud(app),
                        _ => {}
                    }
                });

            // Left-clicking the tray icon is the obvious gesture, but Linux
            // tray backends (appindicator / StatusNotifierItem) never emit
            // TrayIconEvent::Click - there, the menu above is the only way in.
            // Wiring it anyway on Linux would be dead code; leaving it out
            // everywhere would needlessly cost the gesture on Windows/macOS.
            #[cfg(not(target_os = "linux"))]
            let tray = tray.on_tray_icon_event(|tray, event| {
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    toggle_hud(tray.app_handle());
                }
            });

            tray.build(app)?;

            // Click-away dismiss, like a macOS menu-bar popover - handled in
            // the frontend (popover/+page.svelte) instead of here, since it
            // needs to know the collapsed/expanded UI state: the collapsed
            // "mini" pill should stay on screen even when it loses focus.

            // Filesystem watcher: near-instant context-usage updates whenever
            // Claude Code / Codex write to their local transcripts.
            {
                let handle = handle.clone();
                std::thread::spawn(move || {
                    use notify::{RecursiveMode, Watcher};
                    let (tx, rx) = std::sync::mpsc::channel();
                    let Ok(mut watcher) = notify::recommended_watcher(move |res| {
                        let _ = tx.send(res);
                    }) else {
                        return;
                    };
                    for root in context::watch_roots() {
                        let _ = watcher.watch(&root, RecursiveMode::Recursive);
                    }
                    // Claude Code rewrites its transcript continuously, so
                    // reacting to every single event meant re-reading the
                    // file dozens of times a second. Coalesce a burst into
                    // one refresh; the context readout is not worth more
                    // resolution than this.
                    const QUIET_PERIOD: Duration = Duration::from_millis(400);
                    while rx.recv().is_ok() {
                        while rx.recv_timeout(QUIET_PERIOD).is_ok() {}
                        let handle = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            commands::refresh_context_only(&handle).await;
                        });
                    }
                });
            }

            // Periodic network poll for the limit windows. The windows being
            // tracked are hours and days long, so polling every 90s bought
            // nothing and was enough - across restarts - to earn a 429 from
            // Anthropic's endpoint. Five minutes is still far finer-grained
            // than anything it reports.
            {
                let handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    commands::refresh_all(&handle).await;
                    let mut ticker = tokio::time::interval(Duration::from_secs(300));
                    loop {
                        ticker.tick().await;
                        commands::refresh_all(&handle).await;
                    }
                });
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // A tray app outlives its windows. Hiding the HUD (on click-away,
            // or on the blur that follows startup) leaves zero visible
            // windows, which Tauri otherwise treats as "the app is done" and
            // exits with code 0 - no panic, no message, the tray icon just
            // vanishes. `code` is `None` for exactly that window-driven exit
            // and `Some` for a programmatic one, so the tray's "Çıkış" still
            // quits normally.
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
