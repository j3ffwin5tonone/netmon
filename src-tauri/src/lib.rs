mod metrics;

use metrics::{format_tray_title, format_tray_tooltip, AppState, MetricsHistory};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_shell::ShellExt;

const BEER_URL: &str = "https://paypal.me/adSoftware1";

fn is_about_document(url: &tauri::Url, document: &tauri::Url) -> bool {
    let mut target = url.clone();
    target.set_fragment(None);
    &target == document
}

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

            // Build during setup, not inside a synchronous menu handler (WebView2).
            // The offline notices need neither JavaScript nor IPC capabilities.
            // Resolve before page load; querying the main webview can still return about:blank.
            let bundled_origin = if cfg!(target_os = "windows") {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            };
            let base_url = if tauri::is_dev() {
                app.config().build.dev_url.clone()
            } else {
                None
            }
            .unwrap_or(tauri::Url::parse(bundled_origin)?);
            let about_document = base_url.join("licenses.html")?;
            let browser_app = app.handle().clone();
            let about_window =
                WebviewWindowBuilder::new(app, "about", WebviewUrl::App("licenses.html".into()))
                    .title("About netmon & Licenses")
                    .inner_size(760.0, 640.0)
                    .min_inner_size(360.0, 300.0)
                    .visible(false)
                    .disable_javascript()
                    .on_navigation(move |url| {
                        if url.as_str() == BEER_URL {
                            let app = browser_app.clone();
                            tauri::async_runtime::spawn_blocking(move || {
                                // Reuse the installed shell plugin; only this fixed URL is opened.
                                #[allow(deprecated)]
                                if let Err(error) = app.shell().open(BEER_URL, None) {
                                    eprintln!("Could not open the PayPal link: {error}");
                                }
                            });
                            return false;
                        }
                        // Keep license anchors working and reject all other navigation.
                        is_about_document(url, &about_document)
                    })
                    .build()?;
            let hide_about = about_window.clone();
            about_window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = hide_about.hide();
                }
            });

            let about = MenuItem::with_id(app, "about", "About & Licenses", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&about, &separator, &quit])?;

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
                    } else if event.id() == "about" {
                        if let Some(window) = app.get_webview_window("about") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
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

#[cfg(test)]
mod about_tests {
    use super::{is_about_document, BEER_URL};

    #[test]
    fn about_navigation_keeps_the_local_document_and_anchors() {
        for base in [
            "tauri://localhost/licenses.html",
            "http://tauri.localhost/licenses.html",
            "http://localhost:1420/licenses.html",
        ] {
            let document = tauri::Url::parse(base).unwrap();
            assert!(is_about_document(&document, &document));
            assert!(is_about_document(
                &document.join("#dependencies").unwrap(),
                &document
            ));
            for target in [
                BEER_URL,
                "https://example.com/licenses.html",
                "https://paypal.me/adSoftware1?redirect=evil",
                "file:///licenses.html",
            ] {
                assert!(!is_about_document(
                    &tauri::Url::parse(target).unwrap(),
                    &document
                ));
            }
            assert!(!is_about_document(
                &document.join("index.html").unwrap(),
                &document
            ));
        }
    }
}
