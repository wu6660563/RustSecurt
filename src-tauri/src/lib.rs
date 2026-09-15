pub mod commands;
pub mod database;
pub mod file_attributes;
pub mod models;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let database = database::Database::open(app.handle())?;
            let _ = database.recover_operations();
            if let Some(command) = std::env::args().nth(1) {
                if command == "lock" {
                    if let Some(path) = std::env::args().nth(2) {
                        let _ = commands::lock_from_context_menu(path, &database);
                    }
                }
            }
            app.manage(database);
            app.manage(commands::RecoveryScans::default());
            app.manage(commands::AccessSession::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::hide_file,
            commands::hide_folder,
            commands::hide_files,
            commands::hide_folders,
            commands::hide_paths,
            commands::preview_paths,
            commands::restore_item,
            commands::restore_items,
            commands::list_items,
            commands::start_recovery_scan,
            commands::cancel_recovery_scan,
            commands::recover_marked_item,
            commands::password_configured,
            commands::verify_password,
            commands::set_access_password,
            commands::clear_access_password,
            commands::get_auto_lock_minutes,
            commands::set_auto_lock_minutes,
            commands::check_session,
            commands::lock_session,
            commands::health_check,
            commands::delete_history,
            commands::storage_info,
            commands::context_menu_enabled,
            commands::set_context_menu_enabled
        ])
        .run(tauri::generate_context!())
        .expect("启动 FileHide 失败");
}
