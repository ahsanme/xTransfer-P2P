import { useUpdater } from "../../hooks/useUpdater";
import "./UpdateBanner.css";

export function UpdateBanner() {
  const { update, installUpdate, installing, progress, error, dismiss } =
    useUpdater();

  if (!update) return null;

  return (
    <div className="update-banner">
      <div className="update-banner__content">
        <span className="update-banner__icon">⬆</span>
        <span className="update-banner__text">
          <strong>xTransfer {update.version}</strong> is available
        </span>
      </div>

      <div className="update-banner__actions">
        {error && (
          <span className="update-banner__error" title={error}>
            Update failed
          </span>
        )}

        <button
          className="update-banner__btn update-banner__btn--install"
          onClick={installUpdate}
          disabled={installing}
        >
          {installing
            ? progress > 0
              ? `Downloading ${progress}%…`
              : "Downloading…"
            : "Install & Restart"}
        </button>

        {!installing && (
          <button
            className="update-banner__btn update-banner__btn--dismiss"
            onClick={dismiss}
            title="Dismiss"
          >
            ✕
          </button>
        )}
      </div>
    </div>
  );
}
