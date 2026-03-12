use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

/// Commands from Tauri command handlers → swarm event loop
#[derive(Debug)]
pub enum SwarmCommand {
    SendFile {
        peer_id: libp2p::PeerId,
        file_path: PathBuf,
        transfer_id: Uuid,
    },
    AcceptTransfer {
        transfer_id: Uuid,
        save_path: PathBuf,
    },
    RejectTransfer {
        transfer_id: Uuid,
    },
    CancelTransfer {
        transfer_id: Uuid,
    },
    ConnectPeer {
        multiaddr: libp2p::Multiaddr,
    },
    GetConnectionCode {
        reply_tx: tokio::sync::oneshot::Sender<Result<String, String>>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub display_name: String,
    pub source: String, // "lan" | "internet"
    pub connected: bool,
    pub addresses: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransferDirection {
    Send,
    Receive,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    Pending,
    Active,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferInfo {
    pub transfer_id: String,
    pub peer_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub bytes_transferred: u64,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    pub save_path: Option<String>,
    pub error: Option<String>,
}

pub struct AppState {
    /// Send commands into the swarm task
    pub swarm_cmd_tx: mpsc::Sender<SwarmCommand>,
    /// Shared peer list (written by swarm loop, read by commands)
    pub peers: Arc<Mutex<HashMap<libp2p::PeerId, PeerInfo>>>,
    /// Shared transfer state
    pub transfers: Arc<Mutex<HashMap<Uuid, TransferInfo>>>,
    /// Local PeerId string (set once swarm starts)
    pub local_peer_id: Arc<Mutex<String>>,
}

impl AppState {
    /// Returns (AppState, receiver_for_swarm_loop)
    pub fn new() -> (Self, mpsc::Receiver<SwarmCommand>) {
        let (tx, rx) = mpsc::channel::<SwarmCommand>(256);
        let state = AppState {
            swarm_cmd_tx: tx,
            peers: Arc::new(Mutex::new(HashMap::new())),
            transfers: Arc::new(Mutex::new(HashMap::new())),
            local_peer_id: Arc::new(Mutex::new(String::new())),
        };
        (state, rx)
    }
}
