import { downloadDir, join } from "@tauri-apps/api/path";
import { save } from "@tauri-apps/plugin-dialog";
import type { TransferInfo } from "../../lib/types";
import { shortId } from "../../lib/utils";
import { api } from "../../lib/tauri";
import "./IncomingFilePrompt.css";

interface Props {
  transfers: TransferInfo[];
  aliases: Record<string, string>;
  savePath: string | null;
}

export function IncomingFilePrompt({ transfers, aliases, savePath }: Props) {
  const pending = transfers.filter((t) => t.status === "pending" && t.direction === "receive");
  if (pending.length === 0) return null;

  return (
    <div className="incoming-section">
      <h3 className="incoming-section__title">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 10 12 15 17 10" />
          <line x1="12" y1="15" x2="12" y2="3" />
        </svg>
        Incoming Files
        <span className="incoming-section__badge">{pending.length}</span>
      </h3>
      <div className="incoming-section__list">
        {pending.map((t) => (
          <IncomingItem key={t.transfer_id} transfer={t} aliases={aliases} savePath={savePath} />
        ))}
      </div>
    </div>
  );
}

function IncomingItem({
  transfer: t,
  aliases,
  savePath,
}: {
  transfer: TransferInfo;
  aliases: Record<string, string>;
  savePath: string | null;
}) {
  const senderName = aliases[t.peer_id] ?? shortId(t.peer_id);

  const handleAccept = async () => {
    const dir = savePath ?? (await downloadDir());
    const path = await join(dir, t.file_name);
    await api.acceptTransfer(t.transfer_id, path);
  };

  const handleSaveAs = async () => {
    const path = await save({
      defaultPath: t.file_name,
      title: "Save received file",
    });
    if (!path) return;
    await api.acceptTransfer(t.transfer_id, path);
  };

  const handleDecline = async () => {
    await api.rejectTransfer(t.transfer_id);
  };

  return (
    <div className="incoming-item">
      <div className="incoming-item__icon">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
        </svg>
      </div>
      <div className="incoming-item__info">
        <p className="incoming-item__name" title={t.file_name}>{t.file_name}</p>
        <p className="incoming-item__meta">
          {formatBytes(t.file_size)} · from {senderName}
        </p>
      </div>
      <div className="incoming-item__actions">
        <button className="incoming-item__btn incoming-item__btn--accept" onClick={handleAccept}>
          Accept
        </button>
        <button className="incoming-item__btn incoming-item__btn--saveas" onClick={handleSaveAs}>
          Save As
        </button>
        <button className="incoming-item__btn incoming-item__btn--decline" onClick={handleDecline}>
          Decline
        </button>
      </div>
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
