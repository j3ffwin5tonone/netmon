mod metrics;

use metrics::{format_tray_title, format_tray_tooltip, AppState, MetricsHistory};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_shell::ShellExt;

const BEER_URL: &str = "https://paypal.me/adSoftware1";

#[derive(Default)]
struct WindowState {
    dashboard_visible: AtomicBool,
    about_opening: AtomicBool,
}

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

#[tauri::command]
fn set_network_interface(
    name: Option<String>,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .set_network_interface(name)
}

fn show_dashboard(app: &AppHandle) -> tauri::Result<()> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    #[cfg(target_os = "macos")]
    app.show()?;
    win.unminimize()?;
    win.show()?;
    win.set_focus()?;
    app.state::<WindowState>()
        .dashboard_visible
        .store(true, Ordering::Relaxed);
    app.emit_to("main", "dashboard-shown", ())?;
    Ok(())
}

fn request_dashboard(app: &AppHandle) {
    if let Err(error) = show_dashboard(app) {
        eprintln!("Could not show dashboard: {error}");
    }
}

fn build_about(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(window) = app.get_webview_window("about") {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
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
    let browser_app = app.clone();
    // Created on a worker, not a synchronous menu callback: WebView2 requires this.
    // Closing destroys this WebView and releases the large offline notices DOM.
    let window = WebviewWindowBuilder::new(app, "about", WebviewUrl::App("licenses.html".into()))
        .title("About netmon & Licenses")
        .inner_size(760.0, 640.0)
        .min_inner_size(360.0, 300.0)
        .center()
        .disable_javascript()
        .on_navigation(move |url| {
            if url.as_str() == BEER_URL {
                let app = browser_app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    // Reuse the installed plugin; the URL is fixed, never supplied by the page.
                    #[allow(deprecated)]
                    if let Err(error) = app.shell().open(BEER_URL, None) {
                        eprintln!("Could not open the PayPal link: {error}");
                    }
                });
                return false;
            }
            is_about_document(url, &about_document)
        })
        .build()?;
    #[cfg(target_os = "macos")]
    app.show()?;
    window.set_focus()?;
    Ok(())
}

fn request_about(app: &AppHandle) {
    if app
        .state::<WindowState>()
        .about_opening
        .swap(true, Ordering::AcqRel)
    {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = build_about(&app) {
            eprintln!("Could not show About & Licenses: {error}");
        }
        app.state::<WindowState>()
            .about_opening
            .store(false, Ordering::Release);
    });
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(AppState::new()))
        .manage(WindowState::default())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let dashboard =
                MenuItem::with_id(app, "dashboard", "Open Dashboard", true, None::<&str>)?;
            let about = MenuItem::with_id(app, "about", "About & Licenses", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&dashboard, &about, &separator, &quit])?;

            if let Some(win) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                let hide_win = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        match hide_win.hide() {
                            Ok(()) => handle
                                .state::<WindowState>()
                                .dashboard_visible
                                .store(false, Ordering::Relaxed),
                            Err(error) => eprintln!("Could not hide dashboard: {error}"),
                        }
                    }
                });
            }

            let mut tray_builder = TrayIconBuilder::with_id("netmon-tray")
                .menu(&menu)
                .tooltip("netmon — Open Dashboard from this menu")
                .show_menu_on_left_click(false);
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                tray_builder = tray_builder.title("↓ --  ↑ --    --%   --%");
            }
            let _tray = tray_builder
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => app.exit(0),
                    "dashboard" => request_dashboard(app),
                    "about" => request_about(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    // Open after release, once the native status-button gesture has finished.
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        request_dashboard(tray.app_handle());
                    }
                })
                .build(app)?;

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let period = std::time::Duration::from_secs(1);
                let mut timer =
                    tokio::time::interval_at(tokio::time::Instant::now() + period, period);
                timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                let mut last_title = String::new();
                let mut last_tooltip = String::new();
                loop {
                    timer.tick().await;
                    let snapshot = handle
                        .state::<Mutex<AppState>>()
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .tick();
                    if let Some(tray) = handle.tray_by_id("netmon-tray") {
                        let title = format_tray_title(&snapshot);
                        if title != last_title {
                            #[cfg(any(target_os = "macos", target_os = "linux"))]
                            if let Err(error) = tray.set_title(Some(&title)) {
                                eprintln!("Tray title: {error}");
                            }
                            last_title = title;
                        }
                        let tooltip = format_tray_tooltip(&snapshot);
                        if tooltip != last_tooltip {
                            if let Err(error) = tray.set_tooltip(Some(&tooltip)) {
                                eprintln!("Tray tooltip: {error}");
                            }
                            last_tooltip = tooltip;
                        }
                    }
                    if handle
                        .state::<WindowState>()
                        .dashboard_visible
                        .load(Ordering::Relaxed)
                    {
                        if let Some(window) = handle.get_webview_window("main") {
                            // Minimizing can leave the requested visibility flag set.
                            if window.is_visible().unwrap_or(false)
                                && !window.is_minimized().unwrap_or(true)
                            {
                                if let Err(error) =
                                    handle.emit_to("main", "metrics-update", &snapshot)
                                {
                                    eprintln!("Dashboard update: {error}");
                                }
                            }
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_metrics_history,
            set_network_interface
        ])
        .build(tauri::generate_context!())
        .expect("error while building netmon");
    app.run(|app, event| match event {
        tauri::RunEvent::Ready if std::env::args().any(|arg| arg == "--dashboard") => {
            request_dashboard(app)
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => request_dashboard(app),
        _ => {}
    });
}

#[cfg(test)]
mod about_tests {
    use super::{is_about_document, BEER_URL};
    #[test]
    fn about_navigation_keeps_only_the_local_document_and_anchors() {
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
