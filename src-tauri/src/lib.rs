mod commands;
mod state;

use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

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

/// Passed by the autostart entry so that pairdock starts in the tray.
const MINIMIZED_ARG: &str = "--minimized";

/// Set when the previous version installed this one from the window; the
/// installer restarts PairDock with that version's arguments, which may
/// include [`MINIMIZED_ARG`].
static UPDATED: AtomicBool = AtomicBool::new(false);

/// Marks the next start as following an update, see [`UPDATED`].
pub(crate) fn update_marker(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_local_data_dir()
        .ok()
        .map(|dir| dir.join("updated"))
}

/// The page is shown at this zoom so that it fits laptop screens; the window
/// sizes in tauri.conf.json match it.
const UI_ZOOM: f64 = 0.85;

/// How long quitting may take to restore diverted buttons.
pub(crate) const RESTORE_LIMIT: Duration = Duration::from_secs(3);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main_window(app)))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .setup(|app| {
            #[cfg(windows)]
            migrate_legacy_autostart(app.handle());
            // PairDock lives in the menu bar, without a Dock icon or app menu.
            // Info.plist says so too, but tao applies its own default policy
            // once launched.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_zoom(UI_ZOOM);
            }
            if let Some(marker) = update_marker(app.handle()) {
                UPDATED.store(std::fs::remove_file(marker).is_ok(), Ordering::SeqCst);
            }
            let state = AppState::load(app.handle())?;
            app.manage(state);
            build_tray(app.handle())?;
            Ok(())
        })
        // The window starts hidden (see tauri.conf.json) and is shown once the
        // page has loaded, so the webview's blank white surface never shows.
        // When started minimized it stays in the tray, unless it was just
        // updated from the window.
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished
                && (!std::env::args().any(|arg| arg == MINIMIZED_ARG)
                    || UPDATED.load(Ordering::SeqCst))
            {
                let _ = webview.window().show();
                // A menu bar app (LSUIElement in Info.plist) is not brought to
                // the front when it starts.
                #[cfg(target_os = "macos")]
                let _ = webview.window().set_focus();
            }
        })
        .on_window_event(|window, event| {
            // Closing the window keeps pairdock running in the tray; the remapping
            // engine has to stay alive for shortcuts to work.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_receivers,
            commands::device_details,
            commands::device_battery,
            commands::unpair,
            commands::start_pairing,
            commands::pair_device,
            commands::stop_pairing,
            commands::rediscover,
            commands::get_config,
            commands::set_button,
            commands::can_send_keys,
            commands::request_send_keys,
            commands::check_update,
            commands::install_update,
            commands::open_repository,
        ])
        .build(tauri::generate_context!())
        .expect("error while building the pairdock app");

    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
    app.run(|app, event| match event {
        RunEvent::ExitRequested { code: None, api, .. } => {
            // Only the tray's "Quit" ends pairdock (it exits with a code after
            // restoring the buttons). The OS ending the session never gets
            // here: the windows answer WM_QUERYENDSESSION right away and
            // nothing blocks shutting down.
            api.prevent_exit();
        },
        // Opening PairDock again from Finder or Launchpad while it runs.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => show_main_window(app),
        // Logging out or shutting down ends the app directly, without asking
        // first; restore the buttons before the process goes away.
        #[cfg(target_os = "macos")]
        RunEvent::Exit => {
            if let Some(state) = app.try_state::<AppState>() {
                tauri::async_runtime::block_on(state.shutdown(RESTORE_LIMIT));
            }
        },
        _ => {},
    });
}

/// Replaces the autostart entry of the app's pre-rename build ("boltkit"),
/// which points at an executable that no longer exists, with PairDock's own.
#[cfg(windows)]
fn migrate_legacy_autostart(app: &AppHandle) {
    use tauri_plugin_autostart::ManagerExt;

    let Ok(exe) = std::env::current_exe() else { return };
    let Ok(legacy) = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("boltkit")
        .set_app_path(&exe.to_string_lossy())
        .build()
    else {
        return;
    };
    match remove_autostart(&legacy) {
        Ok(true) => {
            if let Err(err) = app.autolaunch().enable() {
                tracing::warn!("could not enable autostart: {err}");
            }
        },
        Ok(false) => {},
        Err(err) => tracing::warn!("could not remove the old autostart entry: {err}"),
    }
}

/// Removes an autostart entry, also one the user disabled in Task Manager,
/// and returns whether it was enabled.
#[cfg(windows)]
fn remove_autostart(entry: &auto_launch::AutoLaunch) -> auto_launch::Result<bool> {
    let enabled = entry.is_enabled()?;
    entry.disable()?;
    Ok(enabled)
}

#[cfg(all(test, windows))]
mod tests {
    use std::process::Command;

    use super::remove_autostart;

    const RUN: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str =
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

    fn reg(args: &[&str]) -> bool {
        Command::new("reg").args(args).output().unwrap().status.success()
    }

    #[test]
    fn removes_an_entry_disabled_in_task_manager() {
        let name = format!("PairDockTest{}", std::process::id());
        let entry = auto_launch::AutoLaunchBuilder::new()
            .set_app_name(&name)
            .set_app_path(r"C:\nonexistent\boltkit.exe")
            .set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser)
            .build()
            .unwrap();
        entry.enable().unwrap();
        // What Task Manager writes when the user disables the entry.
        let disabled = reg(&[
            "add", APPROVED, "/v", &name, "/t", "REG_BINARY", "/d", "030000000000000001000000", "/f",
        ]);

        let result = remove_autostart(&entry);
        let still_there = reg(&["query", RUN, "/v", &name]);
        let _ = entry.disable();
        reg(&["delete", APPROVED, "/v", &name, "/f"]);

        assert!(disabled, "could not mark the entry as disabled");
        assert!(!result.unwrap(), "a disabled entry must not turn PairDock's autostart on");
        assert!(!still_there, "the entry was left in the Run key");
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open PairDock", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit_item])?;

    // A left click opens the window on Windows. On macOS a click on a menu
    // bar extra shows its menu (Human Interface Guidelines, "The menu bar").
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("PairDock")
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => quit(app),
            _ => {},
        })
        .on_tray_icon_event(|tray, event| {
            if !cfg!(target_os = "macos")
                && let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    // The menu bar takes a template image (black and clear) that macOS tints
    // for light and dark menu bars; it is drawn from icons/source/tray-template.svg.
    #[cfg(target_os = "macos")]
    {
        let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
        tray = tray.icon(icon).icon_as_template(true);
    }
    #[cfg(not(target_os = "macos"))]
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
