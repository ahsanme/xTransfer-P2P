import { invoke } from "@tauri-apps/api/core";
import type { PeerInfo, TransferInfo } from "./types";

export const api = {
  getPeerId: () => invoke<string>("get_peer_id"),
  getConnectionCode: () => invoke<string>("get_connection_code"),
  connectPeer: (code: string) => invoke<string>("connect_peer", { code }),
  getPeers: () => invoke<PeerInfo[]>("get_peers"),
  sendFile: (peerId: string, filePath: string) =>
    invoke<string>("send_file", { peerId, filePath }),
  acceptTransfer: (transferId: string, savePath: string) =>
    invoke<void>("accept_transfer", { transferId, savePath }),
  rejectTransfer: (transferId: string) =>
    invoke<void>("reject_transfer", { transferId }),
  cancelTransfer: (transferId: string) =>
    invoke<void>("cancel_transfer", { transferId }),
  getTransfers: () => invoke<TransferInfo[]>("get_transfers"),
};
