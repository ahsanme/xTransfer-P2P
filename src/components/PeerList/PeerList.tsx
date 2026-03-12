import { useRef, useState } from "react";
import type { PeerInfo } from "../../lib/types";
import { shortId } from "../../lib/utils";
import "./PeerList.css";

interface Props {
  peers: PeerInfo[];
  aliases: Record<string, string>;
  onSetAlias: (peerId: string, alias: string) => void;
  selectedPeerId: string | null;
  onSelect: (peerId: string) => void;
}

export function PeerList({ peers, aliases, onSetAlias, selectedPeerId, onSelect }: Props) {
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
              alias={aliases[peer.peer_id] ?? null}
              onSetAlias={onSetAlias}
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
              alias={aliases[peer.peer_id] ?? null}
              onSetAlias={onSetAlias}
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
  alias,
  onSetAlias,
  selected,
  onClick,
}: {
  peer: PeerInfo;
  alias: string | null;
  onSetAlias: (peerId: string, alias: string) => void;
  selected: boolean;
  onClick: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const displayName = alias ?? shortId(peer.peer_id);

  const startEdit = (e: React.MouseEvent) => {
    e.stopPropagation();
    setEditValue(alias ?? "");
    setEditing(true);
    // Focus input on next tick after render
    setTimeout(() => inputRef.current?.focus(), 0);
  };

  const commitEdit = () => {
    onSetAlias(peer.peer_id, editValue);
    setEditing(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") commitEdit();
    if (e.key === "Escape") setEditing(false);
  };

  return (
    <button
      className={`peer-item ${selected ? "peer-item--selected" : ""} ${peer.connected ? "peer-item--connected" : ""}`}
      onClick={onClick}
      title={`${peer.peer_id}\nDouble-click name to set alias`}
    >
      <span className={`peer-item__dot ${peer.connected ? "peer-item__dot--online" : ""}`} />

      {editing ? (
        <input
          ref={inputRef}
          className="peer-item__name-input"
          value={editValue}
          onChange={(e) => setEditValue(e.target.value)}
          onBlur={commitEdit}
          onKeyDown={handleKeyDown}
          onClick={(e) => e.stopPropagation()}
          placeholder={shortId(peer.peer_id)}
          maxLength={24}
        />
      ) : (
        <span
          className="peer-item__name"
          onDoubleClick={startEdit}
          title="Double-click to set alias"
        >
          {displayName}
        </span>
      )}

      <span className="peer-item__badge">{peer.source}</span>
    </button>
  );
}
