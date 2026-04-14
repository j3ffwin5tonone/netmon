use serde::Serialize;
use std::sync::Mutex;
use sysinfo::Networks;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
// --- Types ---

#[derive(Serialize, Clone, Debug)]
pub struct SpeedEntry {
    pub down: f64, // MB/s
    pub up: f64,   // MB/s
}

pub struct NetState {
    networks: Networks,
    history: Vec<SpeedEntry>,
}

// --- Tauri Commands ---

#[tauri::command]
fn get_history(state: State<'_, Mutex<NetState>>) -> Vec<SpeedEntry> {
    state.lock().unwrap().history.clone()
}

// --- App Setup ---
fn format_speed(mb: f64) -> String {
    if mb >= 1.0 {
        format!("{:.1} MB/s", mb)
    } else if mb >= 0.001 {
        format!("{:.0} KB/s", mb * 1024.0)
    } else {
        "0 B/s".to_string()
    }
}
fn measure_speed(state: &Mutex<NetState>) -> SpeedEntry {
    let mut s = state.lock().unwrap();

    // refresh() updates internal counters; received()/transmitted()
    // return bytes since the *previous* refresh — exactly the delta we need.
    s.networks.refresh();

    let (mut down, mut up) = (0u64, 0u64);
    for (name, data) in s.networks.iter() {
        if !name.starts_with("lo") {
            down += data.received();
            up += data.transmitted();
        }
    }

    let entry = SpeedEntry {
        down: down as f64 / 1_048_576.0, // bytes → MB
        up: up as f64 / 1_048_576.0,
    };

    s.history.push(entry.clone());
    if s.history.len() > 60 {
        s.history.remove(0);
    }

    entry
    
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(NetState {
            networks: Networks::new_with_refreshed_list(),
            history: Vec::with_capacity(60),
        }))
        .setup(|app| {
            // --- Tray Menu ---
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            // --- Tray Icon ---
            let _tray = TrayIconBuilder::with_id("netmon-tray")
                .tooltip("Network Monitor")
                .title("↓ 0.0 ↑ 0.0")          // macOS shows this text in the menu bar
                .show_menu_on_left_click(false)
                .menu_on_left_click(false)       // left-click opens window, not menu
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            // --- Background speed polling (every 1 s) ---
            let handle: AppHandle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    let entry = measure_speed(&*handle.state::<Mutex<NetState>>());

                    // Update menu-bar text
                    if let Some(tray) = handle.tray_by_id("netmon-tray") {
                        let _ = tray.set_title(Some(
    &format!("↓ {} ↑ {}", format_speed(entry.down), format_speed(entry.up)),
));
                    }

                    // Push to frontend
                    let _ = handle.emit("network-update", &entry);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_history])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
