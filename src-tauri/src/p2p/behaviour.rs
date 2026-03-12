use crate::p2p::codec::{FileTransferCodec, XFER_PROTOCOL};
use libp2p::{
    autonat, dcutr, identify,
    identity::Keypair,
    kad, mdns, ping, relay,
    request_response::{self, ProtocolSupport},
    swarm::NetworkBehaviour,
};
use std::time::Duration;

/// Bootstrap relay / bootstrap addresses.
/// In production, add your own hosted relay server here.
pub const BOOTSTRAP_RELAYS: &[&str] = &[
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
];

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
