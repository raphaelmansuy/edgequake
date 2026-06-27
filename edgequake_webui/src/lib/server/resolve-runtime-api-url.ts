/**
 * Server-only API base URL for runtime config injection (SPEC-021 P-G13).
 *
 * First principle: the browser must talk to the process that actually serves
 * EdgeQuake `/health`, not a stale Next.js rewrite target or a foreign app on
 * :8080. `make dev` sets EDGEQUAKE_API_URL; bare `bun run dev` does not — we
 * discover the backend the same way as `next.config.ts` rewrites (DRY).
 */

function stripTrailingSlash(url: string): string {
  return url.replace(/\/$/, "");
}

/**
 * Resolve the backend origin injected into `window.__EDGEQUAKE_RUNTIME_CONFIG__`.
 *
 * Development: empty string — browser uses same-origin Next.js rewrites
 * (`/api/v1`, `/live`, `/ws`). Rewrite target discovery lives in `next.config.ts`.
 *
 * Production/docker: trust EDGEQUAKE_API_URL from the container environment.
 */
export function resolveRuntimeApiUrlForInjection(): string {
  // Dev browser traffic must stay same-origin (Next.js rewrites). Injecting
  // http://127.0.0.1:8081 while the UI is on http://localhost:3000 makes
  // fetch/WebSocket cross-origin and falsely triggers "backend not reachable".
  if (process.env.NODE_ENV === "development") {
    return "";
  }

  const envUrl = stripTrailingSlash(
    process.env.EDGEQUAKE_API_URL?.trim() ??
      process.env.NEXT_PUBLIC_API_URL?.trim() ??
      "",
  );

  return envUrl;
}
