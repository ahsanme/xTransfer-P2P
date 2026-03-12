use crate::p2p::behaviour::{AppBehaviour, bootstrap_relays};
use crate::p2p::codec::{FileRequest, FileResponse};
use crate::p2p::encryption;
use crate::p2p::transfer::{
    self, IncomingFilePayload, IncomingTransfer, OutgoingTransfer, TransferCompletePayload,
    TransferFailedPayload, TransferProgressPayload, CHUNK_SIZE,
};
use crate::state::{PeerInfo, SwarmCommand, TransferDirection, TransferInfo, TransferStatus};
use anyhow::Result;
use base64::Engine;
use libp2p::{
    autonat, core::muxing::StreamMuxerBox, identify, mdns, noise, quic, relay,
    request_response,
    swarm::SwarmEvent,
    tcp, yamux, Multiaddr, PeerId, Swarm, Transport,
};
use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

struct ChunkMessage {
    peer_id: PeerId,
    request: FileRequest,
    #[allow(dead_code)]
    transfer_id: Uuid,
}

fn load_or_create_keypair() -> Result<libp2p::identity::Keypair> {
    const SERVICE: &str = "xtransfer-p2p";
    let instance = std::env::var("XTRANSFER_INSTANCE").unwrap_or_default();
    let username = if instance.is_empty() {
        "identity-keypair".to_string()
    } else {
        format!("identity-keypair-{instance}")
    };
    let entry = keyring::Entry::new(SERVICE, &username)?;
    match entry.get_password() {
        Ok(h) => {
            let bytes = hex::decode(&h)?;
            Ok(libp2p::identity::Keypair::from_protobuf_encoding(&bytes)?)
        }
        Err(_) => {
            let kp = libp2p::identity::Keypair::generate_ed25519();
            entry.set_password(&hex::encode(kp.to_protobuf_encoding()?))?;
            tracing::info!("Generated new keypair: {}", kp.public().to_peer_id());
            Ok(kp)
        }
    }
}

pub async fn run_swarm(
    app_handle: AppHandle,
    mut cmd_rx: mpsc::Receiver<SwarmCommand>,
    peers: Arc<Mutex<HashMap<PeerId, PeerInfo>>>,
    transfers: Arc<Mutex<HashMap<Uuid, TransferInfo>>>,
    local_peer_id_slot: Arc<Mutex<String>>,
) -> Result<()> {
    let keypair = load_or_create_keypair().unwrap_or_else(|e| {
        tracing::warn!("Keyring unavailable ({e}), using in-memory keypair");
        libp2p::identity::Keypair::generate_ed25519()
    });

    // ── Swarm built with SwarmBuilder (avoids manual Either mapping) ────────
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_quic()
        .with_other_transport(|key| {
            let noise = noise::Config::new(key)?;
            // Raise the yamux sub-stream ceiling so heavy transfers never hit the default cap.
            let mut yamux_cfg = yamux::Config::default();
            yamux_cfg.set_max_num_streams(1024);
            let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
                .upgrade(libp2p::core::upgrade::Version::V1Lazy)
                .authenticate(noise)
                .multiplex(yamux_cfg)
                .map(|(p, m), _| (p, StreamMuxerBox::new(m)))
                .boxed();
            Ok(transport)
        })?
        .with_relay_client(noise::Config::new, || yamux::Config::default())?
        .with_behaviour(|key, relay_client| {
            AppBehaviour::new(key.public().to_peer_id(), relay_client, key)
                .expect("AppBehaviour construction failed")
        })?
        .with_swarm_config(|cfg| {
            cfg.with_idle_connection_timeout(Duration::from_secs(60))
        })
        .build();

    let local_peer_id = *swarm.local_peer_id();
    *local_peer_id_slot.lock().await = local_peer_id.to_string();
    tracing::info!("Local PeerId: {local_peer_id}");

    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let relays = bootstrap_relays();
    for addr_str in &relays {
        if let Ok(addr) = addr_str.parse::<Multiaddr>() {
            if let Some(peer_id) = addr.iter().find_map(|p| {
                if let libp2p::multiaddr::Protocol::P2p(id) = p {
                    Some(id)
                } else {
                    None
                }
            }) {
                tracing::info!("Bootstrap relay: {addr}");
                swarm.behaviour_mut().kad.add_address(&peer_id, addr);
            }
        }
    }
    if !relays.is_empty() {
        let _ = swarm.behaviour_mut().kad.bootstrap();
    }

    // Small buffer — back-pressure via ChunkAck keeps at most 1 in-flight at a time.
    let (chunk_tx, mut chunk_rx) = mpsc::channel::<ChunkMessage>(4);
    let mut relay_addr: Option<Multiaddr> = None;
    let mut pending_outgoing: HashMap<Uuid, OutgoingTransfer> = HashMap::new();
    let mut pending_incoming: HashMap<Uuid, IncomingTransfer> = HashMap::new();

    loop {
        tokio::select! {
            event = futures::StreamExt::next(&mut swarm) => {
                let Some(event) = event else { break };
                on_swarm_event(
                    event, &mut swarm, &app_handle,
                    &peers, &transfers,
                    &mut relay_addr,
                    &mut pending_outgoing, &mut pending_incoming,
                    &chunk_tx,
                )
                .await;
            }

            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break };
                on_command(
                    cmd, &mut swarm, &app_handle,
                    &transfers,
                    &mut pending_outgoing,
                    &mut pending_incoming,
                    &relay_addr, local_peer_id, &chunk_tx,
                )
                .await;
            }

            Some(msg) = chunk_rx.recv() => {
                swarm.behaviour_mut().xfer.send_request(&msg.peer_id, msg.request);
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn on_swarm_event(
    event: SwarmEvent<crate::p2p::behaviour::AppBehaviourEvent>,
    swarm: &mut Swarm<AppBehaviour>,
    app: &AppHandle,
    peers: &Arc<Mutex<HashMap<PeerId, PeerInfo>>>,
    transfers: &Arc<Mutex<HashMap<Uuid, TransferInfo>>>,
    relay_addr: &mut Option<Multiaddr>,
    pending_outgoing: &mut HashMap<Uuid, OutgoingTransfer>,
    pending_incoming: &mut HashMap<Uuid, IncomingTransfer>,
    chunk_tx: &mpsc::Sender<ChunkMessage>,
) {
    use crate::p2p::behaviour::AppBehaviourEvent;

    match event {
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            tracing::info!("Connected: {peer_id}");
            {
                let mut m = peers.lock().await;
                m.entry(peer_id)
                    .and_modify(|p| p.connected = true)
                    .or_insert(PeerInfo {
                        peer_id: peer_id.to_string(),
                        display_name: short_id(&peer_id),
                        source: "unknown".into(),
                        connected: true,
                        addresses: vec![],
                    });
            }
            let _ = app.emit(
                "peer-connected",
                serde_json::json!({
                    "peer_id": peer_id.to_string(),
                    "display_name": short_id(&peer_id)
                }),
            );
        }

        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            {
                let mut m = peers.lock().await;
                if let Some(p) = m.get_mut(&peer_id) {
                    p.connected = false;
                }
            }
            let _ = app.emit(
                "peer-disconnected",
                serde_json::json!({ "peer_id": peer_id.to_string() }),
            );
        }

        SwarmEvent::NewListenAddr { address, .. } => {
            tracing::info!("Listening on {address}");
            if address.to_string().contains("p2p-circuit") {
                *relay_addr = Some(address);
            }
        }

        SwarmEvent::Behaviour(bev) => match bev {
            AppBehaviourEvent::Mdns(mdns::Event::Discovered(list)) => {
                for (peer_id, addr) in list {
                    swarm.behaviour_mut().kad.add_address(&peer_id, addr.clone());
                    if !swarm.is_connected(&peer_id) {
                        let _ = swarm.dial(addr.clone());
                    }
                    {
                        let mut m = peers.lock().await;
                        m.entry(peer_id).or_insert(PeerInfo {
                            peer_id: peer_id.to_string(),
                            display_name: short_id(&peer_id),
                            source: "lan".into(),
                            connected: false,
                            addresses: vec![addr.to_string()],
                        });
                    }
                    let _ = app.emit(
                        "peer-discovered",
                        serde_json::json!({
                            "peer_id": peer_id.to_string(),
                            "display_name": short_id(&peer_id),
                            "source": "lan"
                        }),
                    );
                }
            }

            AppBehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. }) => {
                for addr in info.listen_addrs {
                    swarm.behaviour_mut().kad.add_address(&peer_id, addr);
                }
            }

            AppBehaviourEvent::Relay(relay::client::Event::ReservationReqAccepted {
                relay_peer_id,
                ..
            }) => {
                tracing::info!("Relay reservation accepted: {relay_peer_id}");
                if let Some(addr) = relay_addr.as_ref() {
                    let _ = app.emit(
                        "relay-connected",
                        serde_json::json!({ "relay_addr": addr.to_string() }),
                    );
                }
            }

            AppBehaviourEvent::Autonat(autonat::Event::StatusChanged { new, .. }) => {
                let status = match &new {
                    autonat::NatStatus::Public(_) => "public",
                    autonat::NatStatus::Private => "private",
                    autonat::NatStatus::Unknown => "unknown",
                };
                tracing::info!("NAT status: {status}");
                let _ =
                    app.emit("nat-status-changed", serde_json::json!({ "status": status }));
                if matches!(new, autonat::NatStatus::Private) {
                    for addr_str in bootstrap_relays() {
                        if let Ok(addr) = addr_str.parse::<Multiaddr>() {
                            let circuit =
                                addr.with(libp2p::multiaddr::Protocol::P2pCircuit);
                            let _ = swarm.listen_on(circuit);
                            break;
                        }
                    }
                }
            }

            AppBehaviourEvent::Xfer(request_response::Event::Message {
                peer,
                message: request_response::Message::Request { request, channel, .. },
                ..
            }) => {
                on_incoming_request(
                    peer, request, channel, swarm, app, transfers, pending_incoming,
                )
                .await;
            }

            AppBehaviourEvent::Xfer(request_response::Event::Message {
                peer,
                message: request_response::Message::Response { response, .. },
                ..
            }) => {
                on_incoming_response(
                    peer, response, app, transfers, pending_outgoing, chunk_tx,
                )
                .await;
            }

            AppBehaviourEvent::Xfer(request_response::Event::OutboundFailure {
                error, ..
            }) => {
                tracing::error!("Outbound transfer failure: {error:?}");
            }

            _ => {}
        },

        SwarmEvent::OutgoingConnectionError { error, .. } => {
            tracing::warn!("Dial error: {error}");
        }

        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
async fn on_command(
    cmd: SwarmCommand,
    swarm: &mut Swarm<AppBehaviour>,
    app: &AppHandle,
    transfers: &Arc<Mutex<HashMap<Uuid, TransferInfo>>>,
    pending_outgoing: &mut HashMap<Uuid, OutgoingTransfer>,
    pending_incoming: &mut HashMap<Uuid, IncomingTransfer>,
    relay_addr: &Option<Multiaddr>,
    local_peer_id: PeerId,
    chunk_tx: &mpsc::Sender<ChunkMessage>,
) {
    match cmd {
        SwarmCommand::ConnectPeer { multiaddr } => {
            tracing::info!("Dialing {multiaddr}");
            if let Err(e) = swarm.dial(multiaddr) {
                tracing::error!("Dial failed: {e}");
            }
        }

        SwarmCommand::SendFile { peer_id, file_path, transfer_id } => {
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let file_size = tokio::fs::metadata(&file_path)
                .await
                .map(|m| m.len())
                .unwrap_or(0);

            {
                let mut t = transfers.lock().await;
                t.insert(
                    transfer_id,
                    TransferInfo {
                        transfer_id: transfer_id.to_string(),
                        peer_id: peer_id.to_string(),
                        file_name: file_name.clone(),
                        file_size,
                        bytes_transferred: 0,
                        direction: TransferDirection::Send,
                        status: TransferStatus::Pending,
                        save_path: None,
                        error: None,
                    },
                );
            }

            // Notify frontend immediately so it can show the outgoing transfer
            // before any progress events arrive.
            let _ = app.emit(
                "outgoing-file",
                IncomingFilePayload {
                    transfer_id: transfer_id.to_string(),
                    peer_id: peer_id.to_string(),
                    file_name: file_name.clone(),
                    file_size,
                },
            );

            let (_ephemeral_secret, ephemeral_pubkey) =
                encryption::generate_ephemeral_keypair();

            match transfer::build_header(
                transfer_id,
                &file_path,
                Some(([0u8; 32], ephemeral_pubkey)),
            )
            .await
            {
                Ok((header, ..)) => {
                    swarm.behaviour_mut().xfer.send_request(&peer_id, header);
                    // Create the per-transfer ack channel used for back-pressure.
                    // The chunk-reader task waits on ack_rx before sending each chunk;
                    // the main loop forwards ChunkAck events to ack_tx.
                    let (ack_tx, ack_rx) = mpsc::channel::<u64>(2);
                    pending_outgoing.insert(
                        transfer_id,
                        OutgoingTransfer {
                            transfer_id,
                            peer_id,
                            file_path,
                            session_key: None,
                            ack_tx,
                            ack_rx: Some(ack_rx),
                        },
                    );
                }
                Err(e) => {
                    tracing::error!("build_header failed: {e}");
                    set_transfer_status(
                        transfers,
                        transfer_id,
                        TransferStatus::Failed,
                        Some(e.to_string()),
                    )
                    .await;
                }
            }
        }

        SwarmCommand::AcceptTransfer { transfer_id, save_path } => {
            if let Some(incoming) = pending_incoming.get_mut(&transfer_id) {
                incoming.save_path = Some(save_path);
            }
        }

        SwarmCommand::RejectTransfer { transfer_id } => {
            set_transfer_status(transfers, transfer_id, TransferStatus::Cancelled, None).await;
        }

        SwarmCommand::CancelTransfer { transfer_id } => {
            pending_outgoing.remove(&transfer_id);
            pending_incoming.remove(&transfer_id);
            set_transfer_status(transfers, transfer_id, TransferStatus::Cancelled, None).await;
            let _ = app.emit(
                "transfer-cancelled",
                serde_json::json!({ "transfer_id": transfer_id.to_string() }),
            );
        }

        SwarmCommand::GetConnectionCode { reply_tx } => {
            let listeners: Vec<Multiaddr> = swarm.listeners().cloned().collect();
            let code = build_code(local_peer_id, relay_addr, &listeners);
            let _ = reply_tx.send(Ok(code.clone()));
            let _ = app.emit(
                "connection-code-ready",
                serde_json::json!({ "code": code }),
            );
        }
    }
}

async fn on_incoming_request(
    peer: PeerId,
    request: FileRequest,
    channel: request_response::ResponseChannel<FileResponse>,
    swarm: &mut Swarm<AppBehaviour>,
    app: &AppHandle,
    transfers: &Arc<Mutex<HashMap<Uuid, TransferInfo>>>,
    pending_incoming: &mut HashMap<Uuid, IncomingTransfer>,
) {
    match request {
        FileRequest::Header {
            transfer_id,
            file_name,
            file_size,
            total_chunks,
            sha256,
            encrypted,
            ephemeral_pubkey,
        } => {
            tracing::info!("Incoming '{file_name}' ({file_size}B) from {peer}");
            {
                let mut t = transfers.lock().await;
                t.insert(
                    transfer_id,
                    TransferInfo {
                        transfer_id: transfer_id.to_string(),
                        peer_id: peer.to_string(),
                        file_name: file_name.clone(),
                        file_size,
                        bytes_transferred: 0,
                        direction: TransferDirection::Receive,
                        status: TransferStatus::Pending,
                        save_path: None,
                        error: None,
                    },
                );
            }

            let _ = app.emit(
                "incoming-file",
                IncomingFilePayload {
                    transfer_id: transfer_id.to_string(),
                    peer_id: peer.to_string(),
                    file_name: file_name.clone(),
                    file_size,
                },
            );

            let save_path = dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(&file_name);

            pending_incoming.insert(
                transfer_id,
                IncomingTransfer {
                    transfer_id,
                    peer_id: peer,
                    file_name,
                    file_size,
                    total_chunks,
                    expected_sha256: sha256,
                    encrypted,
                    session_key: None,
                    save_path: Some(save_path),
                    chunks_received: 0,
                    bytes_received: 0,
                    last_progress_emit: Instant::now(),
                },
            );

            let _ = swarm.behaviour_mut().xfer.send_response(
                channel,
                FileResponse::Accept {
                    transfer_id,
                    resume_from: None,
                },
            );
        }

        FileRequest::Chunk { transfer_id, chunk_index, data, is_last } => {
            if let Some(incoming) = pending_incoming.get_mut(&transfer_id) {
                let save_path = incoming.save_path.clone().unwrap_or_else(|| {
                    dirs::download_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(&incoming.file_name)
                });

                let write_result: anyhow::Result<()> = async {
                    let mut file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .open(&save_path)
                        .await?;
                    file.seek(SeekFrom::Start(chunk_index * CHUNK_SIZE as u64))
                        .await?;
                    let chunk_data = if incoming.encrypted {
                        if let Some(key) = &incoming.session_key {
                            let tb = transfer_id.as_bytes();
                            encryption::decrypt_chunk(
                                key,
                                chunk_index,
                                &[tb[0], tb[1], tb[2], tb[3]],
                                &data,
                            )?
                        } else {
                            data.clone()
                        }
                    } else {
                        data.clone()
                    };
                    file.write_all(&chunk_data).await?;
                    Ok(())
                }
                .await;

                match write_result {
                    Ok(()) => {
                        incoming.chunks_received += 1;
                        incoming.bytes_received += data.len() as u64;

                        let _ = swarm.behaviour_mut().xfer.send_response(
                            channel,
                            FileResponse::ChunkAck { transfer_id, chunk_index },
                        );

                        if incoming.last_progress_emit.elapsed()
                            >= Duration::from_millis(250)
                        {
                            incoming.last_progress_emit = Instant::now();
                            let pct = (incoming.bytes_received as f64
                                / incoming.file_size as f64)
                                * 100.0;
                            let _ = app.emit(
                                "transfer-progress",
                                TransferProgressPayload {
                                    transfer_id: transfer_id.to_string(),
                                    bytes_transferred: incoming.bytes_received,
                                    total_bytes: incoming.file_size,
                                    percent: pct,
                                    speed_bps: 0,
                                },
                            );
                        }

                        {
                            let mut t = transfers.lock().await;
                            if let Some(info) = t.get_mut(&transfer_id) {
                                info.bytes_transferred = incoming.bytes_received;
                                info.status = TransferStatus::Active;
                            }
                        }

                        if is_last {
                            let save = incoming.save_path.clone().unwrap();
                            let expected = incoming.expected_sha256;
                            let app2 = app.clone();
                            let transfers2 = transfers.clone();
                            tokio::spawn(async move {
                                match transfer::sha256_file(&save).await {
                                    Ok(h) if h == expected => {
                                        set_transfer_status(
                                            &transfers2,
                                            transfer_id,
                                            TransferStatus::Complete,
                                            None,
                                        )
                                        .await;
                                        let _ = app2.emit(
                                            "transfer-complete",
                                            TransferCompletePayload {
                                                transfer_id: transfer_id
                                                    .to_string(),
                                                file_path: Some(
                                                    save.to_string_lossy().to_string(),
                                                ),
                                                direction: "receive".to_string(),
                                            },
                                        );
                                    }
                                    Ok(_) => {
                                        let msg = "SHA-256 mismatch".to_string();
                                        set_transfer_status(
                                            &transfers2,
                                            transfer_id,
                                            TransferStatus::Failed,
                                            Some(msg.clone()),
                                        )
                                        .await;
                                        let _ = app2.emit(
                                            "transfer-failed",
                                            TransferFailedPayload {
                                                transfer_id: transfer_id.to_string(),
                                                error: msg,
                                            },
                                        );
                                    }
                                    Err(e) => {
                                        set_transfer_status(
                                            &transfers2,
                                            transfer_id,
                                            TransferStatus::Failed,
                                            Some(e.to_string()),
                                        )
                                        .await;
                                    }
                                }
                            });
                        }
                    }
                    Err(e) => {
                        tracing::error!("Write chunk {chunk_index} failed: {e}");
                        let _ = swarm.behaviour_mut().xfer.send_response(
                            channel,
                            FileResponse::Error {
                                transfer_id,
                                message: e.to_string(),
                            },
                        );
                    }
                }
            }
        }

        FileRequest::Cancel { transfer_id } => {
            pending_incoming.remove(&transfer_id);
            set_transfer_status(transfers, transfer_id, TransferStatus::Cancelled, None).await;
            let _ = app.emit(
                "transfer-cancelled",
                serde_json::json!({ "transfer_id": transfer_id.to_string() }),
            );
        }
    }
}

async fn on_incoming_response(
    peer: PeerId,
    response: FileResponse,
    app: &AppHandle,
    transfers: &Arc<Mutex<HashMap<Uuid, TransferInfo>>>,
    pending_outgoing: &mut HashMap<Uuid, OutgoingTransfer>,
    chunk_tx: &mpsc::Sender<ChunkMessage>,
) {
    match response {
        FileResponse::Accept { transfer_id, resume_from } => {
            tracing::info!("Transfer {transfer_id} accepted by {peer}");
            set_transfer_status(transfers, transfer_id, TransferStatus::Active, None).await;

            if let Some(outgoing) = pending_outgoing.get_mut(&transfer_id) {
                let file_path = outgoing.file_path.clone();
                let session_key = outgoing.session_key;
                // Take the ack receiver — the chunk-reader task owns it from here.
                let ack_rx = outgoing.ack_rx.take().expect("ack_rx already consumed");
                let chunk_tx2 = chunk_tx.clone();
                let app2 = app.clone();
                let transfers2 = transfers.clone();
                let start = resume_from.unwrap_or(0);

                tokio::spawn(async move {
                    if let Err(e) = read_and_send_chunks(
                        &app2,
                        chunk_tx2,
                        transfers2.clone(),
                        transfer_id,
                        peer,
                        file_path,
                        session_key,
                        start,
                        ack_rx,
                    )
                    .await
                    {
                        tracing::error!("Chunk read error: {e}");
                        let _ = app2.emit(
                            "transfer-failed",
                            TransferFailedPayload {
                                transfer_id: transfer_id.to_string(),
                                error: e.to_string(),
                            },
                        );
                        set_transfer_status(
                            &transfers2,
                            transfer_id,
                            TransferStatus::Failed,
                            Some(e.to_string()),
                        )
                        .await;
                    }
                });
            }
        }

        FileResponse::Reject { transfer_id, reason } => {
            tracing::warn!("Transfer {transfer_id} rejected: {reason}");
            pending_outgoing.remove(&transfer_id);
            set_transfer_status(
                transfers,
                transfer_id,
                TransferStatus::Cancelled,
                Some(reason.clone()),
            )
            .await;
            let _ = app.emit(
                "transfer-failed",
                TransferFailedPayload {
                    transfer_id: transfer_id.to_string(),
                    error: format!("Rejected: {reason}"),
                },
            );
        }

        FileResponse::ChunkAck { transfer_id, chunk_index } => {
            tracing::trace!("ChunkAck: {transfer_id} chunk {chunk_index}");
            // Signal the chunk-reader task that the remote peer received and ack'd this chunk.
            // This is the back-pressure gate: the task won't send the next chunk until it
            // receives this signal, keeping at most 1 chunk in-flight at any time.
            if let Some(outgoing) = pending_outgoing.get(&transfer_id) {
                let _ = outgoing.ack_tx.try_send(chunk_index);
            }
        }

        FileResponse::Error { transfer_id, message } => {
            tracing::error!("Transfer {transfer_id} peer error: {message}");
            set_transfer_status(
                transfers,
                transfer_id,
                TransferStatus::Failed,
                Some(message.clone()),
            )
            .await;
            let _ = app.emit(
                "transfer-failed",
                TransferFailedPayload {
                    transfer_id: transfer_id.to_string(),
                    error: message,
                },
            );
        }
    }
}

async fn read_and_send_chunks(
    app: &AppHandle,
    chunk_tx: mpsc::Sender<ChunkMessage>,
    transfers: Arc<Mutex<HashMap<Uuid, TransferInfo>>>,
    transfer_id: Uuid,
    peer_id: PeerId,
    file_path: PathBuf,
    session_key: Option<[u8; 32]>,
    start_chunk: u64,
    mut ack_rx: mpsc::Receiver<u64>,
) -> anyhow::Result<()> {
    let metadata = tokio::fs::metadata(&file_path).await?;
    let file_size = metadata.len();
    let total_chunks = file_size.div_ceil(CHUNK_SIZE as u64);

    let tid_bytes = transfer_id.as_bytes();
    let tid_prefix = [tid_bytes[0], tid_bytes[1], tid_bytes[2], tid_bytes[3]];

    let mut file = tokio::fs::File::open(&file_path).await?;
    if start_chunk > 0 {
        file.seek(SeekFrom::Start(start_chunk * CHUNK_SIZE as u64))
            .await?;
    }

    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut last_emit = Instant::now();
    let start_time = Instant::now();
    let mut bytes_sent = start_chunk * CHUNK_SIZE as u64;

    for chunk_index in start_chunk..total_chunks {
        // ── Back-pressure: wait for the remote peer's ChunkAck before sending the
        // next chunk.  This keeps exactly ONE chunk in-flight at a time, which
        // prevents yamux from exhausting its sub-stream limit on large files.
        // (Skip the wait for the very first chunk we send in this session.)
        if chunk_index > start_chunk {
            match ack_rx.recv().await {
                Some(_acked) => {} // ack received — safe to proceed
                None => {
                    // Channel closed: transfer was cancelled on our side.
                    tracing::debug!("Transfer {transfer_id} ack channel closed — stopping");
                    return Ok(());
                }
            }
        }

        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }

        let data = if let Some(key) = session_key {
            encryption::encrypt_chunk(&key, chunk_index, &tid_prefix, &buf[..n])?
        } else {
            buf[..n].to_vec()
        };

        let is_last = chunk_index == total_chunks - 1;
        chunk_tx
            .send(ChunkMessage {
                peer_id,
                request: FileRequest::Chunk {
                    transfer_id,
                    chunk_index,
                    data,
                    is_last,
                },
                transfer_id,
            })
            .await?;

        bytes_sent += n as u64;

        if last_emit.elapsed() >= Duration::from_millis(250) {
            last_emit = Instant::now();
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                (bytes_sent as f64 / elapsed) as u64
            } else {
                0
            };
            let pct = (bytes_sent as f64 / file_size as f64) * 100.0;
            {
                let mut t = transfers.lock().await;
                if let Some(info) = t.get_mut(&transfer_id) {
                    info.bytes_transferred = bytes_sent;
                }
            }
            let _ = app.emit(
                "transfer-progress",
                TransferProgressPayload {
                    transfer_id: transfer_id.to_string(),
                    bytes_transferred: bytes_sent,
                    total_bytes: file_size,
                    percent: pct,
                    speed_bps: speed,
                },
            );
        }
    }

    set_transfer_status(&transfers, transfer_id, TransferStatus::Complete, None).await;
    {
        let mut t = transfers.lock().await;
        if let Some(info) = t.get_mut(&transfer_id) {
            info.bytes_transferred = file_size;
        }
    }
    let _ = app.emit(
        "transfer-complete",
        TransferCompletePayload {
            transfer_id: transfer_id.to_string(),
            file_path: Some(file_path.to_string_lossy().to_string()),
            direction: "send".into(),
        },
    );
    tracing::info!("Transfer {transfer_id} fully sent");
    Ok(())
}

fn short_id(peer_id: &PeerId) -> String {
    let s = peer_id.to_string();
    s.chars()
        .rev()
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn build_code(
    peer_id: PeerId,
    relay_addr: &Option<Multiaddr>,
    listeners: &[Multiaddr],
) -> String {
    let best = relay_addr
        .as_ref()
        .or_else(|| listeners.first())
        .cloned()
        .unwrap_or_else(|| format!("/p2p/{peer_id}").parse().unwrap());

    let peer_bytes = peer_id.to_bytes();
    let addr_bytes = best.to_vec();

    let mut payload = vec![0x01u8];
    payload.extend_from_slice(&(peer_bytes.len() as u32).to_be_bytes());
    payload.extend_from_slice(&peer_bytes);
    payload.extend_from_slice(&addr_bytes);

    format!(
        "XT-{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&payload)
    )
}

async fn set_transfer_status(
    transfers: &Arc<Mutex<HashMap<Uuid, TransferInfo>>>,
    transfer_id: Uuid,
    status: TransferStatus,
    error: Option<String>,
) {
    let mut t = transfers.lock().await;
    if let Some(info) = t.get_mut(&transfer_id) {
        info.status = status;
        info.error = error;
    }
}
