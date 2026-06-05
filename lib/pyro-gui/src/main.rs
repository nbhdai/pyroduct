#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_daemon_status,
            commands::get_cache_status,
            commands::purge_cache,
            commands::list_active_playbooks,
            commands::start_playbook,
            commands::stop_playbook,
            commands::delete_playbook,
            commands::call_playbook,
            commands::get_capability_interface_spec,
            commands::get_playbook_spec,
            commands::get_playbook_source,
            commands::get_pyroduct_config,
            commands::update_pyroduct_config,
            commands::purge_capabilities_cache,
            commands::purge_playbooks_cache,
            commands::get_playbook_data,
            commands::query_playbook_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
