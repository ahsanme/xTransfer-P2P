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
  const isDark = settings.theme !== "light";

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal settings-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal__header">
          <h2>Settings</h2>
          <button className="modal__close" onClick={onClose}>✕</button>
        </div>

        {/* ── Appearance ──────────────────────────────────────────────────── */}
        <div className="modal__section">
          <label className="modal__label">Appearance</label>
          <div className="settings__theme-row">
            <button
              className={`settings__theme-btn ${isDark ? "settings__theme-btn--active" : ""}`}
              onClick={() => onUpdate({ theme: "dark" })}
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
              </svg>
              Dark
            </button>
            <button
              className={`settings__theme-btn ${!isDark ? "settings__theme-btn--active" : ""}`}
              onClick={() => onUpdate({ theme: "light" })}
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="5" />
                <line x1="12" y1="1" x2="12" y2="3" />
                <line x1="12" y1="21" x2="12" y2="23" />
                <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
                <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
                <line x1="1" y1="12" x2="3" y2="12" />
                <line x1="21" y1="12" x2="23" y2="12" />
                <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
                <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
              </svg>
              Light
            </button>
          </div>
        </div>

        {/* ── Default save folder ─────────────────────────────────────────── */}
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
