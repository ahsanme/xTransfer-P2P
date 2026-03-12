import type { TransferInfo } from "../../lib/types";
import { api } from "../../lib/tauri";
import "./TransferList.css";

interface Props {
  transfers: TransferInfo[];
}

export function TransferList({ transfers }: Props) {
  // Pending receives are handled by IncomingFilePrompt; skip them here
  const visible = transfers.filter((t) => !(t.status === "pending" && t.direction === "receive"));
  if (visible.length === 0) return null;

  return (
    <div className="transfer-list">
      <h3 className="transfer-list__title">Transfers</h3>
      {visible.map((t) => (
        <TransferItem key={t.transfer_id} transfer={t} />
      ))}
    </div>
  );
}

function TransferItem({ transfer: t }: { transfer: TransferInfo }) {
  const pct =
    t.file_size > 0
      ? Math.round((t.bytes_transferred / t.file_size) * 100)
      : 0;

  const size = formatBytes(t.file_size);
  const sent = formatBytes(t.bytes_transferred);
  const speed = t.speed_bps ? formatSpeed(t.speed_bps) : null;

  const isActive = t.status === "active" || t.status === "pending";

  return (
    <div className={`transfer-item transfer-item--${t.status}`}>
      <div className="transfer-item__header">
        <span className="transfer-item__arrow">
          {t.direction === "send" ? "↑" : "↓"}
        </span>
        <span className="transfer-item__name">{t.file_name}</span>
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
