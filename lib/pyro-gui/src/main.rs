#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tracing_subscriber::EnvFilter;

mod commands;

fn main() {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off,pyroduct=info".into()
    })
            }),
        )
        .init();

    tracing::info!("Starting pyro-gui application");

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
