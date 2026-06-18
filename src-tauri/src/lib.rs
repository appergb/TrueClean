//! TrueClean library entry. Wires modules, managed state, and the IPC surface.

pub mod agent;
pub mod cleaning;
pub mod commands;
pub mod error;
pub mod model;
pub mod permissions;
pub mod scanner;
pub mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::default())
        .setup(|app| {
            // Load persisted settings into managed state at startup.
            commands::settings::load_into_state(app.handle());
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running TrueClean");
}
