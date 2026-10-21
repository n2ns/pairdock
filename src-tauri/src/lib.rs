mod commands;
mod state;

use std::time::Duration;

use tauri::{
    AppHandle,
    Manager,
    RunEvent,
    WindowEvent,
    menu::{Menu, MenuItem},
    webview::PageLoadEvent,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::MacosLauncher;

use crate::state::AppState;

/// Passed by the autostart entry so that boltkit starts in the tray.
const MINIMIZED_ARG: &str = "--minimized";

/// How long quitting may take to restore diverted buttons.
const RESTORE_LIMIT: Duration = Duration::from_secs(3);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main_window(app)))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = AppState::load(app.handle())?;
            app.manage(state);
            build_tray(app.handle())?;
            Ok(())
        })
        // The window starts hidden (see tauri.conf.json) and is shown once the
        // page has loaded, so the webview's blank white surface never shows.
        // When started minimized it stays in the tray.
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished
                && !std::env::args().any(|arg| arg == MINIMIZED_ARG)
            {
                let _ = webview.window().show();
            }
        })
        .on_window_event(|window, event| {
            // Closing the window keeps boltkit running in the tray; the remapping
            // engine has to stay alive for shortcuts to work.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_receivers,
            commands::device_details,
            commands::unpair,
            commands::start_pairing,
            commands::pair_device,
            commands::stop_pairing,
            commands::rediscover,
            commands::get_config,
            commands::set_button,
        ])
        .build(tauri::generate_context!())
        .expect("error while building the boltkit app");

    app.run(|_app, event| {
        if let RunEvent::ExitRequested { code: None, api, .. } = event {
            // Only the tray's "Quit" ends boltkit (it exits with a code after
            // restoring the buttons). The OS ending the session never gets
            // here: the windows answer WM_QUERYENDSESSION right away and
            // nothing blocks shutting down.
            api.prevent_exit();
        }
    });
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open BoltKit", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit_item])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("BoltKit")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => quit(app),
            _ => {},
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Restores the buttons (bounded by [`RESTORE_LIMIT`]) and exits.
fn quit(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        app.state::<AppState>().shutdown(RESTORE_LIMIT).await;
        app.exit(0);
    });
}
