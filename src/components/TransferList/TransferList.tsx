import type { TransferInfo } from "../../lib/types";
import { shortId } from "../../lib/utils";
import { api } from "../../lib/tauri";
import "./TransferList.css";

interface Props {
  transfers: TransferInfo[];
  aliases: Record<string, string>;
}

export function TransferList({ transfers, aliases }: Props) {
  // Pending receives are handled by IncomingFilePrompt; skip them here
  const visible = transfers.filter((t) => !(t.status === "pending" && t.direction === "receive"));
  if (visible.length === 0) return null;

  return (
    <div className="transfer-list">
      <h3 className="transfer-list__title">Transfer Log</h3>
      {visible.map((t) => (
        <TransferItem key={t.transfer_id} transfer={t} aliases={aliases} />
      ))}
    </div>
  );
}

function TransferItem({
  transfer: t,
  aliases,
}: {
  transfer: TransferInfo;
  aliases: Record<string, string>;
}) {
  const pct =
    t.file_size > 0
      ? Math.round((t.bytes_transferred / t.file_size) * 100)
      : 0;

  const size = formatBytes(t.file_size);
  const sent = formatBytes(t.bytes_transferred);
  const speed = t.speed_bps ? formatSpeed(t.speed_bps) : null;

  const isActive = t.status === "active" || t.status === "pending";
  const isSend = t.direction === "send";
  const peerName = aliases[t.peer_id] ?? shortId(t.peer_id);

  return (
    <div className={`transfer-item transfer-item--${t.direction} transfer-item--${t.status}`}>
      <div className="transfer-item__header">
        {/* Upload / Download SVG icon */}
        <span className="transfer-item__icon">
          {isSend ? (
            /* Upload icon */
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="17 8 12 3 7 8" />
              <line x1="12" y1="3" x2="12" y2="15" />
            </svg>
          ) : (
            /* Download icon */
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
          )}
        </span>

        <div className="transfer-item__info">
          <span className="transfer-item__name">{t.file_name}</span>
          <span className="transfer-item__peer">
            {isSend ? "To" : "From"}: {peerName}
          </span>
        </div>

        {speed && isActive && (
          <span className="transfer-item__speed">{speed}</span>
        )}
        <span className="transfer-item__status">{t.status}</span>
        {isActive && (
          <button
            className="transfer-item__cancel"
            onClick={() => api.cancelTransfer(t.transfer_id)}
            title="Cancel"
          >
            ✕
          </button>
        )}
      </div>

      {isActive && (
        <>
          <div className="transfer-item__bar">
            <div
              className="transfer-item__bar-fill"
              style={{ width: `${pct}%` }}
            />
          </div>
          <div className="transfer-item__meta">
            <span>
              {sent} / {size}
            </span>
            <span>{pct}%</span>
          </div>
        </>
      )}

      {t.status === "failed" && t.error && (
        <p className="transfer-item__error">{t.error}</p>
      )}

      {t.status === "complete" && t.save_path && (
        <p className="transfer-item__path" title={t.save_path}>{t.save_path}</p>
      )}
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function formatSpeed(bps: number): string {
  if (bps < 1024) return `${bps} B/s`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(0)} KB/s`;
  return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
}
