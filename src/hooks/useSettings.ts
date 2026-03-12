import { useState } from "react";

const SETTINGS_KEY = "xtransfer_settings";

export interface Settings {
  savePath: string | null;
  theme: "dark" | "light";
}

const DEFAULT_SETTINGS: Settings = {
  savePath: null,
  theme: "dark",
};

function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

export function useSettings() {
  const [settings, setSettings] = useState<Settings>(loadSettings);

  const updateSettings = (patch: Partial<Settings>) => {
    setSettings((prev) => {
      const next = { ...prev, ...patch };
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(next));
      return next;
    });
  };

  return { settings, updateSettings };
}
