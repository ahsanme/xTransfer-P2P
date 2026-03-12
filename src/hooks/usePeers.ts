import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { api } from "../lib/tauri";
import type { PeerInfo } from "../lib/types";

export function usePeers() {
  const [peers, setPeers] = useState<PeerInfo[]>([]);

  useEffect(() => {
    api.getPeers().then(setPeers).catch(console.error);

    const unlistenDiscovered = listen<PeerInfo>("peer-discovered", (e) => {
      setPeers((prev) =>
        prev.find((p) => p.peer_id === e.payload.peer_id)
          ? prev
          : [...prev, { ...e.payload, connected: false }]
      );
    });

    const unlistenConnected = listen<{ peer_id: string; display_name: string }>(
      "peer-connected",
      (e) => {
        setPeers((prev) =>
          prev.map((p) =>
            p.peer_id === e.payload.peer_id
              ? { ...p, connected: true }
              : p
          )
        );
      }
    );

    const unlistenDisconnected = listen<{ peer_id: string }>(
      "peer-disconnected",
      (e) => {
        setPeers((prev) =>
          prev.map((p) =>
            p.peer_id === e.payload.peer_id ? { ...p, connected: false } : p
          )
        );
      }
    );

    return () => {
      unlistenDiscovered.then((fn) => fn());
      unlistenConnected.then((fn) => fn());
      unlistenDisconnected.then((fn) => fn());
    };
  }, []);

  return { peers };
}
