/**
 * Size-scaled upload timeouts (SPEC-038 REQ-038-11).
 * Mirrors admit SLA: base + per-MB headroom for transfer + sync BYTEA write.
 */

const BASE_TIMEOUT_MS = 60_000;
const MS_PER_MIB = 8_000;
const MAX_TIMEOUT_MS = 600_000;

/** Compute client XHR timeout from file size in bytes. */
export function uploadTimeoutMs(fileSizeBytes: number): number {
  if (!Number.isFinite(fileSizeBytes) || fileSizeBytes <= 0) {
    return BASE_TIMEOUT_MS;
  }
  const mib = Math.ceil(fileSizeBytes / (1024 * 1024));
  return Math.min(MAX_TIMEOUT_MS, BASE_TIMEOUT_MS + mib * MS_PER_MIB);
}

/** Map transfer bytes to UI progress percent (5–85% band). */
export function transferProgressPercent(loaded: number, total: number): number {
  if (total <= 0) return 10;
  const ratio = Math.min(1, Math.max(0, loaded / total));
  return Math.round(5 + ratio * 80);
}

/** Admit-wait band after all bytes are on the wire (85–92%). */
export const ADMIT_PROGRESS_PERCENT = 90;

export function formatUploadMegabytes(bytes: number): string {
  return (bytes / (1024 * 1024)).toFixed(1);
}
