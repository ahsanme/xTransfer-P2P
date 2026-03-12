use crate::state::{AppState, PeerInfo, SwarmCommand};
use libp2p::Multiaddr;
use tauri::State;
use tokio::sync::oneshot;

#[tauri::command]
pub async fn get_peer_id(state: State<'_, AppState>) -> Result<String, String> {
    let id = state.local_peer_id.lock().await.clone();
    Ok(id)
}

#[tauri::command]
pub async fn get_connection_code(state: State<'_, AppState>) -> Result<String, String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .swarm_cmd_tx
        .send(SwarmCommand::GetConnectionCode { reply_tx })
        .await
        .map_err(|e| format!("swarm channel closed: {e}"))?;
    reply_rx.await.map_err(|_| "reply channel dropped".to_string())?
}

#[tauri::command]
pub async fn connect_peer(state: State<'_, AppState>, code: String) -> Result<String, String> {
    let multiaddr = decode_connection_code(&code)?;
    state
        .swarm_cmd_tx
        .send(SwarmCommand::ConnectPeer { multiaddr })
        .await
        .map_err(|e| format!("swarm channel closed: {e}"))?;
    Ok("dial initiated".to_string())
}

#[tauri::command]
pub async fn get_peers(state: State<'_, AppState>) -> Result<Vec<PeerInfo>, String> {
    let peers = state.peers.lock().await;
    Ok(peers.values().cloned().collect())
}

/// Decode a connection code ("XT-<base64url>") into a Multiaddr.
fn decode_connection_code(code: &str) -> Result<Multiaddr, String> {
    let stripped = code
        .strip_prefix("XT-")
        .ok_or("invalid code: missing XT- prefix")?;
    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        stripped,
    )
    .map_err(|e| format!("base64 decode error: {e}"))?;

    // Format: [1 byte version][rest = multiaddr bytes]
    if bytes.is_empty() {
        return Err("empty connection code payload".to_string());
    }
    let _version = bytes[0];
    let multiaddr_bytes = &bytes[1..];
    Multiaddr::try_from(multiaddr_bytes.to_vec())
        .map_err(|e| format!("invalid multiaddr in code: {e}"))
}
