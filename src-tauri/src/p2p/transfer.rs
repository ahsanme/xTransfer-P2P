use crate::p2p::codec::{FileRequest, FileResponse};
use crate::p2p::encryption;
use crate::state::{TransferDirection, TransferInfo, TransferStatus};
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use uuid::Uuid;

/// 1 MiB chunk size — balanced between memory use and round-trip overhead
pub const CHUNK_SIZE: usize = 1024 * 1024;

/// Progress throttle: emit at most one event per 250ms
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

/// Outgoing transfer session (sender side)
pub struct OutgoingTransfer {
    pub transfer_id: Uuid,
    pub peer_id: libp2p::PeerId,
    pub file_path: PathBuf,
    pub session_key: Option<[u8; 32]>,
}

/// Incoming transfer session (receiver side)
pub struct IncomingTransfer {
    pub transfer_id: Uuid,
    pub peer_id: libp2p::PeerId,
    pub file_name: String,
    pub file_size: u64,
    pub total_chunks: u64,
    pub expected_sha256: [u8; 32],
    pub encrypted: bool,
    pub session_key: Option<[u8; 32]>,
    pub save_path: Option<PathBuf>,
    pub chunks_received: u64,
    pub bytes_received: u64,
    pub last_progress_emit: Instant,
}

/// Compute SHA-256 of an entire file (streaming)
pub async fn sha256_file(path: &PathBuf) -> Result<[u8; 32]> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; CHUNK_SIZE];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().into())
}

/// Build a FileRequest::Header for an outgoing file.
/// Returns (header request, file metadata) without starting the actual send loop.
pub async fn build_header(
    transfer_id: Uuid,
    file_path: &PathBuf,
    session_key_and_pubkey: Option<([u8; 32], [u8; 32])>,
) -> Result<(FileRequest, u64, u64)> {
    let metadata = tokio::fs::metadata(file_path).await?;
    let file_size = metadata.len();
    let total_chunks = file_size.div_ceil(CHUNK_SIZE as u64);
    let sha256 = sha256_file(file_path).await?;

    let (encrypted, ephemeral_pubkey) = match session_key_and_pubkey {
        Some((_, pubkey)) => (true, Some(pubkey)),
        None => (false, None),
    };

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let header = FileRequest::Header {
        transfer_id,
        file_name,
        file_size,
        total_chunks,
        sha256,
        encrypted,
        ephemeral_pubkey,
    };

    Ok((header, file_size, total_chunks))
}

/// Emit type for transfer progress events
#[derive(serde::Serialize, Clone)]
pub struct TransferProgressPayload {
    pub transfer_id: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub percent: f64,
    pub speed_bps: u64,
}

/// Emit type for transfer complete events
#[derive(serde::Serialize, Clone)]
pub struct TransferCompletePayload {
    pub transfer_id: String,
    pub file_path: Option<String>,
    pub direction: String,
}

/// Emit type for incoming file notification
#[derive(serde::Serialize, Clone)]
pub struct IncomingFilePayload {
    pub transfer_id: String,
    pub peer_id: String,
    pub file_name: String,
    pub file_size: u64,
}

/// Emit type for transfer failure
#[derive(serde::Serialize, Clone)]
pub struct TransferFailedPayload {
    pub transfer_id: String,
    pub error: String,
}
