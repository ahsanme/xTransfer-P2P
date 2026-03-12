mod commands;
mod p2p;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("xtransfer_p2p_lib=debug".parse().unwrap())
                .add_directive("libp2p=info".parse().unwrap()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let handle = app.handle().clone();

            let (app_state, cmd_rx) = state::AppState::new();

            // Clones for the swarm task (before app_state is moved into manage)
            let peers = app_state.peers.clone();
            let transfers = app_state.transfers.clone();
            let local_peer_id_slot = app_state.local_peer_id.clone();

            app.manage(app_state);

            // Spawn the P2P swarm loop as a background task
            tauri::async_runtime::spawn(async move {
                if let Err(e) = p2p::swarm::run_swarm(
                    handle,
                    cmd_rx,
                    peers,
                    transfers,
                    local_peer_id_slot,
                )
                .await
                {
                    tracing::error!("Swarm task exited with error: {e:#}");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::network::get_peer_id,
            commands::network::get_connection_code,
            commands::network::connect_peer,
            commands::network::get_peers,
            commands::transfer::send_file,
            commands::transfer::accept_transfer,
            commands::transfer::reject_transfer,
            commands::transfer::cancel_transfer,
            commands::transfer::get_transfers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
