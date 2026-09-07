pub mod commands;
pub mod database;
pub mod file_attributes;
pub mod models;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(database::Database::open(app.handle())?);
            app.manage(commands::RecoveryScans::default());
            app.manage(commands::AccessSession::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::hide_file,
            commands::hide_folder,
            commands::restore_item,
            commands::list_items,
            commands::start_recovery_scan,
            commands::cancel_recovery_scan,
            commands::recover_marked_item,
            commands::password_configured,
            commands::verify_password,
            commands::set_access_password,
            commands::clear_access_password
        ])
        .run(tauri::generate_context!())
        .expect("启动 FileHide 失败");
}
