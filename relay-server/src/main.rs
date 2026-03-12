use anyhow::Result;
use futures::StreamExt;
use libp2p::{
    core::muxing::StreamMuxerBox,
    identify, noise,
    relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, Transport,
};
use std::{fs, path::Path, time::Duration};
use tracing::info;

#[derive(NetworkBehaviour)]
struct RelayBehaviour {
    relay:    relay::Behaviour,
    identify: identify::Behaviour,
    ping:     libp2p::ping::Behaviour,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        )
        .init();

    let keypair = load_or_create_keypair("relay-identity.bin")?;
    let peer_id = keypair.public().to_peer_id();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4001);

    // Relay config — generous limits for a dedicated server
    let relay_cfg = relay::Config {
        max_reservations: 1024,
        max_reservations_per_peer: 8,
        reservation_duration: Duration::from_secs(3600),
        max_circuits: 1024,
        max_circuits_per_peer: 16,
        ..Default::default()
    };

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_quic()
        .with_other_transport(|key| {
            let noise = noise::Config::new(key)?;
            let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
                .upgrade(libp2p::core::upgrade::Version::V1Lazy)
                .authenticate(noise)
                .multiplex(yamux::Config::default())
                .map(|(p, m), _| (p, StreamMuxerBox::new(m)))
                .boxed();
            Ok(transport)
        })?
        .with_behaviour(|key: &libp2p::identity::Keypair| RelayBehaviour {
            relay: relay::Behaviour::new(peer_id, relay_cfg),
            identify: identify::Behaviour::new(identify::Config::new(
                "/xtransfer-relay/1.0.0".into(),
                key.public(),
            )),
            ping: libp2p::ping::Behaviour::default(),
        })?
        .with_swarm_config(|cfg: libp2p::swarm::Config| {
            cfg.with_idle_connection_timeout(Duration::from_secs(60))
        })
        .build();

    let tcp_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{port}").parse()?;
    let quic_addr: Multiaddr = format!("/ip4/0.0.0.0/udp/{port}/quic-v1").parse()?;
    swarm.listen_on(tcp_addr)?;
    swarm.listen_on(quic_addr)?;

    // Print startup info — the user needs the PeerID to configure the client
    println!();
    println!("╔══════════════════════════════════════════╗");
    println!("║       xTransfer Relay Server v0.1        ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("  PeerID : {peer_id}");
    println!("  TCP    : 0.0.0.0:{port}");
    println!("  QUIC   : 0.0.0.0:{port}");
    println!();
    println!("  Add to app BOOTSTRAP_RELAYS:");
    println!("    /ip4/<YOUR_PUBLIC_IP>/tcp/{port}/p2p/{peer_id}");
    println!();

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {address}");
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected: {peer_id}");
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                info!("Disconnected: {peer_id} — {cause:?}");
            }
            SwarmEvent::Behaviour(RelayBehaviourEvent::Relay(event)) => {
                info!("Relay event: {event:?}");
            }
            _ => {}
        }
    }
}

fn load_or_create_keypair(path: &str) -> Result<libp2p::identity::Keypair> {
    if Path::new(path).exists() {
        let bytes = fs::read(path)?;
        let keypair = libp2p::identity::Keypair::from_protobuf_encoding(&bytes)?;
        info!("Loaded existing keypair from {path}");
        Ok(keypair)
    } else {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let bytes = keypair.to_protobuf_encoding()?;
        fs::write(path, &bytes)?;
        info!("Generated new keypair, saved to {path}");
        Ok(keypair)
    }
}
