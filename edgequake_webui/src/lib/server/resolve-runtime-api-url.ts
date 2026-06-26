/**
 * Server-only API base URL for runtime config injection (SPEC-021 P-G13).
 *
 * First principle: the browser must talk to the process that actually serves
 * EdgeQuake `/health`, not a stale Next.js rewrite target or a foreign app on
 * :8080. `make dev` sets EDGEQUAKE_API_URL; bare `bun run dev` does not — we
 * discover the backend the same way as `next.config.ts` rewrites (DRY).
 */

import { resolveDevProxyBackend } from "./dev-proxy-backend";

function stripTrailingSlash(url: string): string {
  return url.replace(/\/$/, "");
}

/**
 * Resolve the backend origin injected into `window.__EDGEQUAKE_RUNTIME_CONFIG__`.
 *
 * Development: always run health-validated discovery so the client can bypass
 * Next rewrites that were baked at `next dev` startup (backend may have moved
 * to :8081 or not been up yet).
 *
 * Production/docker: trust EDGEQUAKE_API_URL from the container environment.
 */
export function resolveRuntimeApiUrlForInjection(): string {
  const envUrl = stripTrailingSlash(
    process.env.EDGEQUAKE_API_URL?.trim() ??
      process.env.NEXT_PUBLIC_API_URL?.trim() ??
      "",
  );

  if (process.env.NODE_ENV !== "development") {
    return envUrl;
  }

  try {
    const discovered = stripTrailingSlash(resolveDevProxyBackend());
    return discovered || envUrl;
  } catch {
    return envUrl;
  }
}
