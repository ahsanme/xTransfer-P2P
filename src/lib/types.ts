export interface PeerInfo {
  peer_id: string;
  display_name: string;
  source: "lan" | "internet" | "unknown";
  connected: boolean;
  addresses: string[];
}

export interface TransferInfo {
  transfer_id: string;
  peer_id: string;
  file_name: string;
  file_size: number;
  bytes_transferred: number;
  direction: "send" | "receive";
  status: "pending" | "active" | "complete" | "failed" | "cancelled";
  save_path?: string;
  error?: string;
  speed_bps?: number;
}

export interface TransferProgressEvent {
  transfer_id: string;
  bytes_transferred: number;
  total_bytes: number;
  percent: number;
  speed_bps: number;
}

export interface TransferCompleteEvent {
  transfer_id: string;
  file_path?: string;
  direction: "send" | "receive";
}

export interface TransferFailedEvent {
  transfer_id: string;
  error: string;
}

export interface IncomingFileEvent {
  transfer_id: string;
  peer_id: string;
  file_name: string;
  file_size: number;
}
