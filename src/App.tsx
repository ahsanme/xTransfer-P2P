import { useEffect, useState } from "react";
import { ConnectModal } from "./components/ConnectModal/ConnectModal";
import { DropZone } from "./components/DropZone/DropZone";
import { IncomingFilePrompt } from "./components/IncomingFilePrompt/IncomingFilePrompt";
import { PeerList } from "./components/PeerList/PeerList";
import { TransferList } from "./components/TransferList/TransferList";
import { usePeers } from "./hooks/usePeers";
import { useTransfers } from "./hooks/useTransfers";
import { api } from "./lib/tauri";
import "./App.css";

export function App() {
  const { peers } = usePeers();
  const { transfers } = useTransfers();
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);
  const [showConnect, setShowConnect] = useState(false);
  const [myCode, setMyCode] = useState("");
  const [peerId, setPeerId] = useState("");

  useEffect(() => {
    api.getPeerId().then(setPeerId).catch(console.error);
  }, []);

  const handleShowConnect = async () => {
    setShowConnect(true);
    try {
      const code = await api.getConnectionCode();
      setMyCode(code);
    } catch (e) {
      console.error("Failed to get connection code:", e);
    }
  };

  return (
    <div className="app">
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="sidebar__logo">
          <span className="sidebar__logo-icon">⚡</span>
          <span className="sidebar__logo-text">xTransfer</span>
        </div>
        <PeerList
          peers={peers}
          selectedPeerId={selectedPeerId}
          onSelect={setSelectedPeerId}
        />
        <div className="sidebar__footer">
          <button className="sidebar__connect-btn" onClick={handleShowConnect}>
            + Connect via Code
          </button>
          <p className="sidebar__peer-id" title={peerId}>
            ID: {peerId ? peerId.slice(-8) : "…"}
          </p>
        </div>
      </aside>

      {/* Main content */}
      <main className="main">
        <IncomingFilePrompt transfers={transfers} />
        <div className="main__top">
          <DropZone
            selectedPeerId={selectedPeerId}
            onTransferStarted={() => {}}
          />
        </div>
        <TransferList transfers={transfers} />
      </main>

      {showConnect && (
        <ConnectModal myCode={myCode} onClose={() => setShowConnect(false)} />
      )}
    </div>
  );
}
