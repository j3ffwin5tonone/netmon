mod metrics;

use metrics::{format_speed, AppState, MetricsHistory};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

#[tauri::command]
fn get_metrics_history(state: State<'_, Mutex<AppState>>) -> MetricsHistory {
    state.lock().unwrap().history_snapshot()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(AppState::new()))
        .setup(|app| {
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
            let _tray = TrayIconBuilder::with_id("netmon-tray")
                .menu(&menu)
                .tooltip("Network Monitor")
                .title("↓ 0.0 ↑ 0.0")
                .show_menu_on_left_click(false)
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
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            vis_for_tray.store(true, Ordering::Relaxed);
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            let handle: AppHandle = app.handle().clone();
            let vis_for_loop = Arc::clone(&window_visible);
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    // Mutex only covers sysinfo refreshes + history; GPU is lock-free last-value.
                    let snapshot = {
                        let state = handle.state::<Mutex<AppState>>();
                        let mut s = state.lock().unwrap();
                        s.tick()
                    };

                    if let Some(tray) = handle.tray_by_id("netmon-tray") {
                        let title = if snapshot.gpu_supported {
                            format!(
                                "↓ {} ↑ {} · CPU {:.0}% · RAM {:.0}% · GPU {:.0}%",
                                format_speed(snapshot.network.down),
                                format_speed(snapshot.network.up),
                                snapshot.cpu_percent,
                                snapshot.memory_percent,
                                snapshot.gpu_percent
                            )
                        } else {
                            format!(
                                "↓ {} ↑ {} · CPU {:.0}% · RAM {:.0}%",
                                format_speed(snapshot.network.down),
                                format_speed(snapshot.network.up),
                                snapshot.cpu_percent,
                                snapshot.memory_percent
                            )
                        };
                        let _ = tray.set_title(Some(&title));
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
