use crate::state::{AppState, SwarmCommand, TransferInfo};
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn send_file(
    state: State<'_, AppState>,
    peer_id: String,
    file_path: String,
) -> Result<String, String> {
    let peer_id: libp2p::PeerId = peer_id
        .parse()
        .map_err(|e| format!("invalid peer id: {e}"))?;
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("file not found: {file_path}"));
    }
    let transfer_id = Uuid::new_v4();

    state
        .swarm_cmd_tx
        .send(SwarmCommand::SendFile {
            peer_id,
            file_path: path,
            transfer_id,
        })
        .await
        .map_err(|e| format!("swarm channel closed: {e}"))?;

    Ok(transfer_id.to_string())
}

#[tauri::command]
pub async fn accept_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
    save_path: String,
) -> Result<(), String> {
    let id: Uuid = transfer_id
        .parse()
        .map_err(|e| format!("invalid transfer id: {e}"))?;
    state
        .swarm_cmd_tx
        .send(SwarmCommand::AcceptTransfer {
            transfer_id: id,
            save_path: PathBuf::from(save_path),
        })
        .await
        .map_err(|e| format!("swarm channel closed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn reject_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<(), String> {
    let id: Uuid = transfer_id
        .parse()
        .map_err(|e| format!("invalid transfer id: {e}"))?;
    state
        .swarm_cmd_tx
        .send(SwarmCommand::RejectTransfer { transfer_id: id })
        .await
        .map_err(|e| format!("swarm channel closed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn cancel_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<(), String> {
    let id: Uuid = transfer_id
        .parse()
        .map_err(|e| format!("invalid transfer id: {e}"))?;
    state
        .swarm_cmd_tx
        .send(SwarmCommand::CancelTransfer { transfer_id: id })
        .await
        .map_err(|e| format!("swarm channel closed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn get_transfers(state: State<'_, AppState>) -> Result<Vec<TransferInfo>, String> {
    let transfers = state.transfers.lock().await;
    Ok(transfers.values().cloned().collect())
}
