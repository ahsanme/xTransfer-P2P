use crate::p2p::codec::{FileTransferCodec, XFER_PROTOCOL};
use libp2p::{
    autonat, dcutr, identify,
    identity::Keypair,
    kad, mdns, ping, relay,
    request_response::{self, ProtocolSupport},
    swarm::NetworkBehaviour,
};
use std::time::Duration;

/// Built-in relay address — your hosted AWS relay server.
/// Override at runtime with the XTRANSFER_RELAY env var:
///   XTRANSFER_RELAY=/ip4/1.2.3.4/tcp/4001/p2p/<PeerID>
const BUILTIN_RELAY: &str =
    "/ip4/54.177.30.255/tcp/4001/p2p/12D3KooWAPMCMr36wFaSuP1omKgESYUFdog9Hmg3r5hBU5TUyDLN";

/// Returns the relay addresses to use: env var override, or the built-in.
pub fn bootstrap_relays() -> Vec<String> {
    if let Ok(addr) = std::env::var("XTRANSFER_RELAY") {
        if !addr.is_empty() {
            return vec![addr];
        }
    }
    // Only include the built-in if it looks like it has been configured
    // (i.e. doesn't contain the placeholder text).
    if BUILTIN_RELAY.contains("RELAY_IP") || BUILTIN_RELAY.contains("RELAY_PEER_ID") {
        vec![]
    } else {
        vec![BUILTIN_RELAY.to_string()]
    }
}

/// All behaviours composed via the NetworkBehaviour derive macro.
#[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    pub ping: ping::Behaviour,
    pub identify: identify::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub relay: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub autonat: autonat::Behaviour,
    pub kad: kad::Behaviour<kad::store::MemoryStore>,
    pub xfer: request_response::Behaviour<FileTransferCodec>,
}

impl AppBehaviour {
    pub fn new(
        local_peer_id: libp2p::PeerId,
        relay_client: relay::client::Behaviour,
        keypair: &Keypair,
    ) -> anyhow::Result<Self> {
        let ping = ping::Behaviour::new(
            ping::Config::new()
                .with_interval(Duration::from_secs(20))
                .with_timeout(Duration::from_secs(10)),
        );

        let identify = identify::Behaviour::new(identify::Config::new(
            "/xtransfer/1.0.0".to_string(),
            keypair.public(),
        ));

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;
        let dcutr = dcutr::Behaviour::new(local_peer_id);

        let autonat = autonat::Behaviour::new(
            local_peer_id,
            autonat::Config {
                retry_interval: Duration::from_secs(30),
                refresh_interval: Duration::from_secs(120),
                boot_delay: Duration::from_secs(5),
                ..Default::default()
            },
        );

        let mut kad_cfg = kad::Config::default();
        kad_cfg.set_query_timeout(Duration::from_secs(30));
        let kad = kad::Behaviour::with_config(
            local_peer_id,
            kad::store::MemoryStore::new(local_peer_id),
            kad_cfg,
        );

        let xfer = request_response::Behaviour::new(
            [(XFER_PROTOCOL, ProtocolSupport::Full)],
            request_response::Config::default()
                .with_max_concurrent_streams(8)
                .with_request_timeout(Duration::from_secs(600)),
        );

        Ok(AppBehaviour {
            ping,
            identify,
            mdns,
            relay: relay_client,
            dcutr,
            autonat,
            kad,
            xfer,
        })
    }
}
