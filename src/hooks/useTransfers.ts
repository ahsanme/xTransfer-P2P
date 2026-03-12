import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { api } from "../lib/tauri";
import type {
  IncomingFileEvent,
  TransferCompleteEvent,
  TransferFailedEvent,
  TransferInfo,
  TransferProgressEvent,
} from "../lib/types";

async function notify(title: string, body: string) {
  try {
    let permitted = await isPermissionGranted();
    if (!permitted) {
      const result = await requestPermission();
      permitted = result === "granted";
    }
    if (permitted) sendNotification({ title, body });
  } catch {
    // Notifications not available — silently skip
  }
}

export function useTransfers() {
  const [transfers, setTransfers] = useState<Record<string, TransferInfo>>({});

  const updateTransfer = (id: string, patch: Partial<TransferInfo>) => {
    setTransfers((prev) =>
      prev[id] ? { ...prev, [id]: { ...prev[id], ...patch } } : prev
    );
  };

  useEffect(() => {
    api.getTransfers().then((list) => {
      const map: Record<string, TransferInfo> = {};
      list.forEach((t) => { map[t.transfer_id] = t; });
      setTransfers(map);
    });

    const unIncoming = listen<IncomingFileEvent>("incoming-file", (e) => {
      const t: TransferInfo = {
        transfer_id: e.payload.transfer_id,
        peer_id: e.payload.peer_id,
        file_name: e.payload.file_name,
        file_size: e.payload.file_size,
        bytes_transferred: 0,
        direction: "receive",
        status: "pending",
      };
      setTransfers((prev) => ({ ...prev, [t.transfer_id]: t }));
    });

    // Mirrors incoming-file but for sends — backend emits this right after
    // inserting the transfer into its map so the UI shows it immediately.
    const unOutgoing = listen<IncomingFileEvent>("outgoing-file", (e) => {
      const t: TransferInfo = {
        transfer_id: e.payload.transfer_id,
        peer_id: e.payload.peer_id,
        file_name: e.payload.file_name,
        file_size: e.payload.file_size,
        bytes_transferred: 0,
        direction: "send",
        status: "pending",
      };
      setTransfers((prev) => ({ ...prev, [t.transfer_id]: t }));
    });

    const unProgress = listen<TransferProgressEvent>("transfer-progress", (e) => {
      updateTransfer(e.payload.transfer_id, {
        bytes_transferred: e.payload.bytes_transferred,
        status: "active",
        speed_bps: e.payload.speed_bps,
      });
    });

    const unComplete = listen<TransferCompleteEvent>("transfer-complete", (e) => {
      updateTransfer(e.payload.transfer_id, {
        status: "complete",
        save_path: e.payload.file_path,
        speed_bps: undefined,
      });
      notify(
        "Transfer complete",
        e.payload.direction === "receive"
          ? `File saved to ${e.payload.file_path ?? "disk"}`
          : "File sent successfully"
      );
    });

    const unFailed = listen<TransferFailedEvent>("transfer-failed", (e) => {
      updateTransfer(e.payload.transfer_id, {
        status: "failed",
        error: e.payload.error,
        speed_bps: undefined,
      });
    });

    const unCancelled = listen<{ transfer_id: string }>(
      "transfer-cancelled",
      (e) => {
        updateTransfer(e.payload.transfer_id, { status: "cancelled", speed_bps: undefined });
      }
    );

    return () => {
      unIncoming.then((fn) => fn());
      unOutgoing.then((fn) => fn());
      unProgress.then((fn) => fn());
      unComplete.then((fn) => fn());
      unFailed.then((fn) => fn());
      unCancelled.then((fn) => fn());
    };
  }, []);

  return { transfers: Object.values(transfers) };
}
