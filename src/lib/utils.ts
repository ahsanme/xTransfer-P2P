/**
 * Deterministic 9-digit short ID from a libp2p PeerId string.
 * Returns a human-readable "XXX XXX XXX" format.
 * Uses djb2 hash with Math.imul for reliable 32-bit arithmetic.
 */
export function shortId(peerId: string): string {
  let hash = 5381;
  for (let i = 0; i < peerId.length; i++) {
    hash = Math.imul(hash, 33) ^ peerId.charCodeAt(i);
  }
  const num = Math.abs(hash) % 1_000_000_000;
  const s = num.toString().padStart(9, "0");
  return `${s.slice(0, 3)} ${s.slice(3, 6)} ${s.slice(6, 9)}`;
}
