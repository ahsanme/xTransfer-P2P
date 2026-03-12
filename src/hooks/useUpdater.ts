import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useEffect, useState } from "react";

export function useUpdater() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  // Check for updates once on mount (silent — never throws to the user)
  useEffect(() => {
    check()
      .then((u) => {
        if (u?.available) setUpdate(u);
      })
      .catch(() => {
        // No network / no latest.json yet — silently ignore
      });
  }, []);

  const installUpdate = async () => {
    if (!update || installing) return;
    setInstalling(true);
    setError(null);
    try {
      let downloaded = 0;
      let total = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (total > 0) setProgress(Math.round((downloaded / total) * 100));
            break;
          case "Finished":
            setProgress(100);
            break;
        }
      });
      await relaunch();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setInstalling(false);
    }
  };

  const dismiss = () => setDismissed(true);

  return {
    update: dismissed ? null : update,
    installUpdate,
    installing,
    progress,
    error,
    dismiss,
  };
}
