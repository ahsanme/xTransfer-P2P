import { open } from "@tauri-apps/plugin-dialog";
import type { Settings } from "../../hooks/useSettings";
import "./Settings.css";

interface Props {
  settings: Settings;
  onUpdate: (patch: Partial<Settings>) => void;
  onClose: () => void;
}

export function Settings({ settings, onUpdate, onClose }: Props) {
  const handleBrowse = async () => {
    const selected = await open({ directory: true, title: "Choose default save folder" });
    if (typeof selected === "string" && selected) {
      onUpdate({ savePath: selected });
    }
  };

  const handleReset = () => {
    onUpdate({ savePath: null });
  };

  const displayPath = settings.savePath ?? "~/Downloads (default)";

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal settings-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal__header">
          <h2>Settings</h2>
          <button className="modal__close" onClick={onClose}>✕</button>
        </div>

        <div className="modal__section">
          <label className="modal__label">Default Save Folder</label>
          <p className="settings__hint">
            Received files are automatically saved here when you click "Accept".
          </p>
          <div className="settings__path-row">
            <span className="settings__path" title={settings.savePath ?? undefined}>
              {displayPath}
            </span>
            <button className="modal__btn modal__btn--secondary" onClick={handleBrowse}>
              Browse…
            </button>
          </div>
          {settings.savePath && (
            <button className="settings__reset-link" onClick={handleReset}>
              Reset to Downloads
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
