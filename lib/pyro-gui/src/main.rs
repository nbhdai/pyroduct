#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tracing_subscriber::EnvFilter;

mod commands;

fn main() {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "trace,cranelift_frontend=off,cranelift_codegen=off,mio=off,tao=off,wasmtime=off,pyroduct=info".into()
        }))
        .init();

    tracing::info!("Starting pyro-gui application");

    // When the embedded-daemon feature is enabled, spawn the daemon in-process
    // using ~/.pyroduct as the working directory.
    #[cfg(feature = "embedded-daemon")]
    {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let working_dir = std::path::PathBuf::from(&home).join(".pyroduct");

        // Set env vars so the rest of the codebase (pyro-artifacts cache, daemon
        // client lookups, etc.) all resolve to ~/.pyroduct.
        // SAFETY: This runs at startup before any other threads are spawned,
        // so mutating the environment is safe.
        unsafe {
            std::env::set_var("PYRO_DAEMON_DIR", &working_dir);
            std::env::set_var("PYRODUCT", &working_dir);
        }

        tracing::info!(
            working_dir = %working_dir.display(),
            "Embedded daemon enabled — spawning PyroDaemon in-process"
        );

        let daemon_working_dir = working_dir.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create Tokio runtime for embedded daemon");
            rt.block_on(async move {
                let daemon = pyro_daemon::PyroDaemon::new(daemon_working_dir.clone());
                let socket_path = daemon.control_socket_path.clone();

                // Clean up socket on Ctrl+C
                let socket_cleanup = socket_path.clone();
                tokio::spawn(async move {
                    tokio::signal::ctrl_c()
                        .await
                        .expect("Failed to listen for ctrl-c signal");
                    tracing::info!(
                        "Received shutdown signal. Cleaning up embedded daemon socket..."
                    );
                    if socket_cleanup.exists() {
                        let _ = std::fs::remove_file(&socket_cleanup);
                    }
                    std::process::exit(0);
                });

                if let Err(e) = daemon.run().await {
                    tracing::error!("Embedded daemon exited with error: {:?}", e);
                }
            });
        });

        // Brief pause to let the daemon bind its socket before the GUI starts
        // querying it.
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_gui_settings,
            commands::update_gui_settings,
            commands::run_bulk_playbook,
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
            commands::query_playbook_data,
            commands::get_playbook_failures,
            commands::get_playbook_execution_record,
            commands::list_sessions,
            commands::set_http_address
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
