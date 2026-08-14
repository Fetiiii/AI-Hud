mod commands;
mod context;
mod credentials;
mod providers;
mod state;

use state::AppState;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::refresh_now,
            commands::open_detail,
            commands::hide_popover,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let quit_item = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id().as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("popover") {
                            let visible = win.is_visible().unwrap_or(false);
                            if visible {
                                let _ = win.hide();
                            } else {
                                // Anchor the popover just under the tray icon,
                                // the way a macOS menu-bar app would.
                                let (icon_x, icon_y) = match rect.position {
                                    tauri::Position::Physical(p) => (p.x, p.y),
                                    tauri::Position::Logical(p) => (p.x as i32, p.y as i32),
                                };
                                let icon_height = match rect.size {
                                    tauri::Size::Physical(s) => s.height as i32,
                                    tauri::Size::Logical(s) => s.height as i32,
                                };
                                let _ = win.set_position(tauri::Position::Physical(
                                    tauri::PhysicalPosition {
                                        x: icon_x,
                                        y: icon_y + icon_height,
                                    },
                                ));
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Click-away dismiss, like a macOS menu-bar popover.
            if let Some(popover) = app.get_webview_window("popover") {
                let popover_handle = popover.clone();
                popover.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        let _ = popover_handle.hide();
                    }
                });
            }

            // The detail window is expensive to recreate, so closing it (the
            // OS window-close button) just hides it - the app keeps living in
            // the tray, same as the popover.
            if let Some(detail) = app.get_webview_window("detail") {
                let detail_handle = detail.clone();
                detail.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = detail_handle.hide();
                    }
                });
            }

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
                    for event in rx {
                        if event.is_ok() {
                            let handle = handle.clone();
                            tauri::async_runtime::spawn(async move {
                                commands::refresh_context_only(&handle).await;
                            });
                        }
                    }
                });
            }

            // Periodic network poll for the 5h/weekly windows. Kept
            // infrequent since it's a live API call, even though it's free.
            {
                let handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    commands::refresh_all(&handle).await;
                    let mut ticker = tokio::time::interval(Duration::from_secs(90));
                    loop {
                        ticker.tick().await;
                        commands::refresh_all(&handle).await;
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
