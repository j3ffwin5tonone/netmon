mod metrics;

use metrics::{format_tray_title, format_tray_tooltip, AppState, MetricsHistory};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

#[tauri::command]
fn get_metrics_history(state: State<'_, Mutex<AppState>>) -> MetricsHistory {
    state
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .history_snapshot()
}

fn show_dashboard(app: &AppHandle, window_visible: &AtomicBool) {
    if let Some(win) = app.get_webview_window("main") {
        window_visible.store(true, Ordering::Relaxed);
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(AppState::new()))
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            let window_visible = Arc::new(AtomicBool::new(false));

            if let Some(win) = app.get_webview_window("main") {
                let vis = Arc::clone(&window_visible);
                let hide_win = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = hide_win.hide();
                        vis.store(false, Ordering::Relaxed);
                    }
                });
            }

            let vis_for_tray = Arc::clone(&window_visible);
            let mut tray_builder = TrayIconBuilder::with_id("netmon-tray")
                .menu(&menu)
                .tooltip("Network Monitor")
                .show_menu_on_left_click(false);

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            // Live tray title text is macOS (and partially Linux) only; Windows ignores it.
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                tray_builder = tray_builder.title("↓  0B ↑  0B   0%   0%");
            }

            let _tray = tray_builder
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_dashboard(tray.app_handle(), &vis_for_tray);
                    }
                })
                .build(app)?;

            let handle: AppHandle = app.handle().clone();
            let vis_for_loop = Arc::clone(&window_visible);
            tauri::async_runtime::spawn(async move {
                let mut last_title = String::new();
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    // Mutex only covers sysinfo refreshes + history; GPU is lock-free last-value.
                    let snapshot = {
                        let state = handle.state::<Mutex<AppState>>();
                        let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
                        s.tick()
                    };

                    if let Some(tray) = handle.tray_by_id("netmon-tray") {
                        let title = format_tray_title(&snapshot);
                        // Skip AppKit setTitle when unchanged — width churn is what macOS
                        // uses to cull status items from a crowded menu bar.
                        if title != last_title {
                            #[cfg(any(target_os = "macos", target_os = "linux"))]
                            {
                                let _ = tray.set_title(Some(&title));
                            }
                            let _ = tray.set_tooltip(Some(format_tray_tooltip(&snapshot)));
                            last_title = title;
                        }
                    }

                    if vis_for_loop.load(Ordering::Relaxed) {
                        let _ = handle.emit("metrics-update", &snapshot);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_metrics_history])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
