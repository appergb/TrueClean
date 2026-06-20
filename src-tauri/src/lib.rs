//! TrueClean library entry. Wires modules, managed state, and the IPC surface.

pub mod agent;
pub mod cleaning;
pub mod commands;
pub mod error;
pub mod model;
pub mod permissions;
pub mod scanner;
pub mod secrets;
pub mod state;

use state::AppState;

// Windows 专属：注入与顶栏融合的自定义窗口控件（保留 Snap 布局）。
// macOS 红绿灯位置由 tauri.conf.json 的 trafficLightPosition 控制。
#[cfg(target_os = "windows")]
use tauri::Manager;
#[cfg(target_os = "windows")]
use tauri_plugin_decorum::WebviewWindowExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_decorum::init())
        .manage(AppState::default())
        .setup(|app| {
            // Load persisted settings into managed state at startup.
            commands::settings::load_into_state(app.handle());

            // Windows：隐藏原生标题栏，注入与顶栏融合的自定义窗口控件
            // （保留 Snap 布局）。macOS 红绿灯位置由 tauri.conf.json 的
            // trafficLightPosition 控制，无需在此处理。
            #[cfg(target_os = "windows")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.create_overlay_titlebar();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // scan
            commands::scan::get_volumes,
            commands::scan::scan_path,
            commands::scan::cancel_scan,
            // cleanup
            commands::cleanup::scan_junk,
            commands::cleanup::find_large_old_files,
            commands::cleanup::clean_paths,
            commands::cleanup::empty_trash,
            commands::cleanup::restore_last_clean,
            // system extras
            commands::system::find_duplicates,
            commands::system::list_applications,
            commands::system::uninstall_app,
            commands::system::list_startup_items,
            commands::system::set_startup_item,
            // agent
            commands::agent::agent_chat,
            commands::agent::agent_cancel,
            commands::agent::agent_confirm,
            // settings
            commands::settings::get_settings,
            commands::settings::save_settings,
            // permissions
            commands::permissions::get_permission_status,
            commands::permissions::open_system_permission_settings,
            commands::permissions::get_helper_status,
            commands::permissions::install_privileged_helper,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TrueClean");
}
