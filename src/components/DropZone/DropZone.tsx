import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api } from "../../lib/tauri";
import "./DropZone.css";

interface Props {
  selectedPeerId: string | null;
  onTransferStarted: (transferId: string) => void;
}

export function DropZone({ selectedPeerId, onTransferStarted }: Props) {
  const [dragging, setDragging] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  const sendFiles = async (paths: string[]) => {
    if (!selectedPeerId) {
      setStatus("Select a peer first");
      setTimeout(() => setStatus(null), 3000);
      return;
    }
    for (const path of paths) {
      try {
        const id = await api.sendFile(selectedPeerId, path);
        onTransferStarted(id);
      } catch (e) {
        // Show the backend error message directly — it already contains a human-readable
        // explanation (e.g. for .app bundles: "folder or app bundle cannot be sent…")
        const msg = typeof e === "string" ? e : `Send failed: ${e}`;
        setStatus(msg);
        setTimeout(() => setStatus(null), 7000);
      }
    }
  };

  // Tauri drag-drop event (gives real OS file paths)
  useEffect(() => {
    const unlisten = listen<{ paths: string[]; position: unknown }>(
      "tauri://drag-drop",
      (e) => {
        setDragging(false);
        sendFiles(e.payload.paths);
      }
    );
    const unHover = listen("tauri://drag-enter", () => setDragging(true));
    const unLeave = listen("tauri://drag-leave", () => setDragging(false));
    return () => {
      unlisten.then((fn) => fn());
      unHover.then((fn) => fn());
      unLeave.then((fn) => fn());
    };
  }, [selectedPeerId]);

  const handleClick = async () => {
    const selected = await open({ multiple: true });
    if (!selected) return;
    const paths: string[] = Array.isArray(selected) ? selected : [selected as string];
    await sendFiles(paths);
  };

  return (
    <div
      className={`dropzone ${dragging ? "dropzone--dragging" : ""} ${!selectedPeerId ? "dropzone--disabled" : ""}`}
      onClick={handleClick}
    >
      <div className="dropzone__icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="17 8 12 3 7 8" />
          <line x1="12" y1="3" x2="12" y2="15" />
        </svg>
      </div>
      {status ? (
        <p className="dropzone__status dropzone__status--error">{status}</p>
      ) : selectedPeerId ? (
        <>
          <p className="dropzone__primary">Drop files here</p>
          <p className="dropzone__secondary">or click to browse</p>
        </>
      ) : (
        <>
          <p className="dropzone__primary">Select a peer</p>
          <p className="dropzone__secondary">then drop files to send</p>
        </>
      )}
    </div>
  );
}
