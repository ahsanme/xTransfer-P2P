import { useEffect, useState } from "react";
import { ConnectModal } from "./components/ConnectModal/ConnectModal";
import { DropZone } from "./components/DropZone/DropZone";
import { IncomingFilePrompt } from "./components/IncomingFilePrompt/IncomingFilePrompt";
import { PeerList } from "./components/PeerList/PeerList";
import { Settings } from "./components/Settings/Settings";
import { TransferList } from "./components/TransferList/TransferList";
import { UpdateBanner } from "./components/UpdateBanner/UpdateBanner";
import { usePeerAliases } from "./hooks/usePeerAliases";
import { usePeers } from "./hooks/usePeers";
import { useSettings } from "./hooks/useSettings";
import { useTransfers } from "./hooks/useTransfers";
import { api } from "./lib/tauri";
import { shortId } from "./lib/utils";
import "./App.css";

export function App() {
  const { peers } = usePeers();
  const { transfers } = useTransfers();
  const { settings, updateSettings } = useSettings();
  const { aliases, setAlias } = usePeerAliases();
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);
  const [showConnect, setShowConnect] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [myCode, setMyCode] = useState("");
  const [peerId, setPeerId] = useState("");

  // Apply theme to <html> whenever it changes (covers initial load too)
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", settings.theme ?? "dark");
  }, [settings.theme]);

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
          aliases={aliases}
          onSetAlias={setAlias}
          selectedPeerId={selectedPeerId}
          onSelect={setSelectedPeerId}
        />
        <div className="sidebar__footer">
          <button className="sidebar__connect-btn" onClick={handleShowConnect}>
            + Connect via Code
          </button>
          <div className="sidebar__bottom-row">
            <p className="sidebar__peer-id" title={peerId}>
              {peerId ? shortId(peerId) : "…"}
            </p>
            <button
              className="sidebar__settings-btn"
              onClick={() => setShowSettings(true)}
              title="Settings"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
            </button>
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main className="main">
        <UpdateBanner />
        <IncomingFilePrompt
          transfers={transfers}
          aliases={aliases}
          savePath={settings.savePath}
        />
        <div className="main__top">
          <DropZone
            selectedPeerId={selectedPeerId}
            onTransferStarted={() => {}}
          />
        </div>
        <TransferList transfers={transfers} aliases={aliases} />
      </main>

      {showConnect && (
        <ConnectModal myCode={myCode} onClose={() => setShowConnect(false)} />
      )}

      {showSettings && (
        <Settings
          settings={settings}
          onUpdate={updateSettings}
          onClose={() => setShowSettings(false)}
        />
      )}
    </div>
  );
}
