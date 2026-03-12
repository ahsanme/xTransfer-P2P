import { useState } from "react";
import { api } from "../../lib/tauri";
import "./ConnectModal.css";

interface Props {
  myCode: string;
  onClose: () => void;
}

export function ConnectModal({ myCode, onClose }: Props) {
  const [code, setCode] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleConnect = async () => {
    if (!code.trim()) return;
    setLoading(true);
    setStatus(null);
    try {
      await api.connectPeer(code.trim());
      setStatus("Connecting...");
      setTimeout(onClose, 1500);
    } catch (e) {
      setStatus(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const copyCode = () => {
    navigator.clipboard.writeText(myCode);
    setStatus("Copied!");
    setTimeout(() => setStatus(null), 2000);
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal__header">
          <h2>Connect to Peer</h2>
          <button className="modal__close" onClick={onClose}>✕</button>
        </div>

        <div className="modal__section">
          <label className="modal__label">Your Connection Code</label>
          <div className="modal__code-row">
            <code className="modal__code">{myCode || "Loading..."}</code>
            <button className="modal__btn modal__btn--secondary" onClick={copyCode}>
              Copy
            </button>
          </div>
          <p className="modal__hint">Share this code with the other person</p>
        </div>

        <div className="modal__divider">— or —</div>

        <div className="modal__section">
          <label className="modal__label">Enter Their Code</label>
          <input
            className="modal__input"
            type="text"
            placeholder="XT-..."
            value={code}
            onChange={(e) => setCode(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleConnect()}
          />
          <button
            className="modal__btn modal__btn--primary"
            onClick={handleConnect}
            disabled={loading || !code.trim()}
          >
            {loading ? "Connecting..." : "Connect"}
          </button>
        </div>

        {status && <p className="modal__status">{status}</p>}
      </div>
    </div>
  );
}
