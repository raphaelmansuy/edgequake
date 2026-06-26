/**
 * @module upload-limits
 * @description Single source of truth for client-side upload limits.
 *
 * WHY: The backend enforces `DefaultBodyLimit::max(resource_budget.max_upload_bytes)`
 * (see `edgequake-api/src/server.rs`). The default is 50 MiB in
 * `edgequake-core/src/resource/budget.rs` (`MAX_UPLOAD_BYTES`), overridable via
 * `EDGEQUAKE_MAX_UPLOAD_BYTES`. The frontend must reject oversized files
 * *before* upload to avoid a confusing server-side 413, so it needs its own
 * cap that mirrors the backend.
 *
 * Coupling rule: when the backend default changes, update
 * `DEFAULT_MAX_UPLOAD_BYTES` here to match. The value can also be overridden at
 * build/runtime via `NEXT_PUBLIC_MAX_UPLOAD_BYTES` for deployments that raise
 * the backend limit.
 */

/**
 * Default maximum upload size in bytes (50 MiB).
 *
 * Mirrors `MAX_UPLOAD_BYTES` in `edgequake-core/src/resource/budget.rs`.
 * Keep in sync when the backend default changes.
 */
export const DEFAULT_MAX_UPLOAD_BYTES = 50 * 1024 * 1024;

/**
 * Effective maximum upload size in bytes.
 *
 * Resolved from (in priority order):
 * 1. `NEXT_PUBLIC_MAX_UPLOAD_BYTES` env var (set at build time for deployments
 *    that raise the backend limit via `EDGEQUAKE_MAX_UPLOAD_BYTES`).
 * 2. {@link DEFAULT_MAX_UPLOAD_BYTES}.
 *
 * Clamped to a minimum of 1 MiB to prevent misconfiguration from breaking
 * uploads entirely.
 */
export const MAX_UPLOAD_BYTES: number = (() => {
  const raw = process.env.NEXT_PUBLIC_MAX_UPLOAD_BYTES;
  if (!raw) return DEFAULT_MAX_UPLOAD_BYTES;
  const parsed = Number.parseInt(raw, 10);
  if (!Number.isFinite(parsed) || parsed < 1024 * 1024) {
    return DEFAULT_MAX_UPLOAD_BYTES;
  }
  return parsed;
})();

/** Human-readable limit for UI strings (e.g., "max 50MB"). */
export const MAX_UPLOAD_LABEL = `${Math.round(MAX_UPLOAD_BYTES / (1024 * 1024))}MB`;
