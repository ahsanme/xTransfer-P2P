use async_trait::async_trait;
use futures::prelude::*;
use libp2p::{request_response, StreamProtocol};
use serde::{Deserialize, Serialize};
use std::io;
use uuid::Uuid;

/// Protocol identifier (libp2p 0.55+: StreamProtocol, not ProtocolName)
pub const XFER_PROTOCOL: StreamProtocol = StreamProtocol::new("/xtransfer/file/1.0.0");

/// Requests sent from the initiating peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileRequest {
    Header {
        transfer_id: Uuid,
        file_name: String,
        file_size: u64,
        total_chunks: u64,
        sha256: [u8; 32],
        encrypted: bool,
        ephemeral_pubkey: Option<[u8; 32]>,
    },
    Chunk {
        transfer_id: Uuid,
        chunk_index: u64,
        data: Vec<u8>,
        is_last: bool,
    },
    Cancel {
        transfer_id: Uuid,
    },
}

/// Responses from the receiving peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileResponse {
    Accept {
        transfer_id: Uuid,
        resume_from: Option<u64>,
    },
    Reject {
        transfer_id: Uuid,
        reason: String,
    },
    ChunkAck {
        transfer_id: Uuid,
        chunk_index: u64,
    },
    Error {
        transfer_id: Uuid,
        message: String,
    },
}

/// Codec for length-prefix-framed JSON messages
#[derive(Debug, Clone, Default)]
pub struct FileTransferCodec;

#[async_trait]
impl request_response::Codec for FileTransferCodec {
    type Protocol = StreamProtocol;
    type Request = FileRequest;
    type Response = FileResponse;

    async fn read_request<T>(&mut self, _: &StreamProtocol, io: &mut T) -> io::Result<FileRequest>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_lp_json(io).await
    }

    async fn read_response<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<FileResponse>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_lp_json(io).await
    }

    async fn write_request<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
        req: FileRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_lp_json(io, &req).await
    }

    async fn write_response<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
        resp: FileResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_lp_json(io, &resp).await
    }
}

async fn read_lp_json<T, D>(io: &mut T) -> io::Result<D>
where
    T: AsyncRead + Unpin + Send,
    D: for<'de> serde::Deserialize<'de>,
{
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 128 * 1024 * 1024 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
    }
    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

async fn write_lp_json<T, S>(io: &mut T, value: &S) -> io::Result<()>
where
    T: AsyncWrite + Unpin + Send,
    S: serde::Serialize,
{
    let bytes =
        serde_json::to_vec(value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    io.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
    io.write_all(&bytes).await?;
    io.flush().await
}
