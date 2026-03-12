import { useState } from "react";

const ALIASES_KEY = "xtransfer_peer_aliases";

function loadAliases(): Record<string, string> {
  try {
    const raw = localStorage.getItem(ALIASES_KEY);
    if (!raw) return {};
    return JSON.parse(raw) as Record<string, string>;
  } catch {
    return {};
  }
}

export function usePeerAliases() {
  const [aliases, setAliases] = useState<Record<string, string>>(loadAliases);

  const setAlias = (peerId: string, alias: string) => {
    const trimmed = alias.trim();
    setAliases((prev) => {
      const next = { ...prev };
      if (trimmed) {
        next[peerId] = trimmed;
      } else {
        delete next[peerId];
      }
      localStorage.setItem(ALIASES_KEY, JSON.stringify(next));
      return next;
    });
  };

  const clearAlias = (peerId: string) => {
    setAliases((prev) => {
      const next = { ...prev };
      delete next[peerId];
      localStorage.setItem(ALIASES_KEY, JSON.stringify(next));
      return next;
    });
  };

  return { aliases, setAlias, clearAlias };
}
