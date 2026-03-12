import type { PeerInfo } from "../../lib/types";
import "./PeerList.css";

interface Props {
  peers: PeerInfo[];
  selectedPeerId: string | null;
  onSelect: (peerId: string) => void;
}

export function PeerList({ peers, selectedPeerId, onSelect }: Props) {
  const connected = peers.filter((p) => p.connected);
  const discovered = peers.filter((p) => !p.connected);

  return (
    <div className="peer-list">
      <h2 className="peer-list__title">Peers</h2>
      {peers.length === 0 && (
        <p className="peer-list__empty">Searching for peers on your network...</p>
      )}
      {connected.length > 0 && (
        <section>
          <p className="peer-list__section-label">Connected</p>
          {connected.map((peer) => (
            <PeerItem
              key={peer.peer_id}
              peer={peer}
              selected={peer.peer_id === selectedPeerId}
              onClick={() => onSelect(peer.peer_id)}
            />
          ))}
        </section>
      )}
      {discovered.length > 0 && (
        <section>
          <p className="peer-list__section-label">Discovered</p>
          {discovered.map((peer) => (
            <PeerItem
              key={peer.peer_id}
              peer={peer}
              selected={peer.peer_id === selectedPeerId}
              onClick={() => onSelect(peer.peer_id)}
            />
          ))}
        </section>
      )}
    </div>
  );
}

function PeerItem({
  peer,
  selected,
  onClick,
}: {
  peer: PeerInfo;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={`peer-item ${selected ? "peer-item--selected" : ""} ${peer.connected ? "peer-item--connected" : ""}`}
      onClick={onClick}
    >
      <span className={`peer-item__dot ${peer.connected ? "peer-item__dot--online" : ""}`} />
      <span className="peer-item__name">{peer.display_name}</span>
      <span className="peer-item__badge">{peer.source}</span>
    </button>
  );
}
