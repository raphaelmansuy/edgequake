export interface EdgeQuakeRuntimeConfig {
  apiUrl: string;
  authEnabled: boolean;
  disableDemoLogin: boolean;
  /**
   * When true, the login page shows and prefills the well-known local-dev
   * credentials (`admin` / `EdgeQuake1`). Set only by `make dev` — never in prod.
   */
  showDevLoginHint: boolean;
  /**
   * Periodic `/live`+`/health` poll interval. `false` (default) = one probe on
   * mount. Set `EDGEQUAKE_HEALTH_POLL_MS` (e.g. `10000`) to restore looping.
   */
  healthPollIntervalMs: number | false;
}

declare global {
  interface Window {
    __EDGEQUAKE_RUNTIME_CONFIG__?: Partial<EdgeQuakeRuntimeConfig>;
  }
}

function parseBoolean(value: string | boolean | undefined | null): boolean {
  if (typeof value === 'boolean') {
    return value;
  }

  const normalized = value?.toString().trim().toLowerCase();
  return normalized === 'true' || normalized === '1' || normalized === 'yes' || normalized === 'on';
}

/** unset / 0 / false / off → no periodic poll; a positive integer is the interval in ms. */
export function parseHealthPollIntervalMs(value: unknown): number | false {
  if (typeof value === "number") {
    if (!Number.isFinite(value) || value <= 0) {
      return false;
    }
    return Math.floor(value);
  }
  if (value === false || value === null || value === undefined) {
    return false;
  }
  if (typeof value === "boolean") {
    return false;
  }
  const normalized = String(value).trim().toLowerCase();
  if (
    normalized === "" ||
    normalized === "0" ||
    normalized === "false" ||
    normalized === "off" ||
    normalized === "no"
  ) {
    return false;
  }
  const parsed = Number(normalized);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return false;
  }
  return Math.floor(parsed);
}

function resolveClientApiUrl(
  browserConfig: Partial<EdgeQuakeRuntimeConfig> | undefined,
): string {
  if (browserConfig?.apiUrl !== undefined) {
    return browserConfig.apiUrl.replace(/\/$/, "");
  }
  // Local dev: same-origin Next.js rewrites (/api/v1, /live, /ws). EDGEQUAKE_API_URL
  // is server-only (rewrite target + SSR); the browser must not call it directly.
  if (process.env.NODE_ENV === "development") {
    return "";
  }
  return (
    process.env.EDGEQUAKE_API_URL ??
    process.env.NEXT_PUBLIC_API_URL ??
    ""
  ).replace(/\/$/, "");
}

export function getRuntimeConfig(): EdgeQuakeRuntimeConfig {
  const browserConfig = typeof window !== 'undefined' ? window.__EDGEQUAKE_RUNTIME_CONFIG__ : undefined;

  // WHY EDGEQUAKE_API_URL (not NEXT_PUBLIC_API_URL):
  // NEXT_PUBLIC_* variables are inlined at build time by the Next.js compiler.
  // This means the image always carries the build-time default (http://localhost:8080)
  // and cannot be overridden at container startup — breaking custom EDGEQUAKE_PORT
  // deployments and remote-access setups.
  //
  // EDGEQUAKE_API_URL is a plain (non-NEXT_PUBLIC_) env var that Next.js server
  // components read from the actual process environment at request time.
  // layout.tsx (server component) calls getRuntimeConfig() and injects the result
  // into window.__EDGEQUAKE_RUNTIME_CONFIG__ so the client picks it up without
  // a build-time bake. The NEXT_PUBLIC_API_URL fallback is kept for local dev
  // (where .env.local may define it) and backwards compatibility.
  return {
    apiUrl: resolveClientApiUrl(browserConfig),
    authEnabled: parseBoolean(browserConfig?.authEnabled ?? process.env.NEXT_PUBLIC_AUTH_ENABLED),
    disableDemoLogin: parseBoolean(
      browserConfig?.disableDemoLogin ?? process.env.NEXT_PUBLIC_DISABLE_DEMO_LOGIN
    ),
    showDevLoginHint: parseBoolean(
      browserConfig?.showDevLoginHint ?? process.env.NEXT_PUBLIC_SHOW_DEV_LOGIN_HINT
    ),
    healthPollIntervalMs: parseHealthPollIntervalMs(
      browserConfig?.healthPollIntervalMs !== undefined
        ? browserConfig.healthPollIntervalMs
        : process.env.EDGEQUAKE_HEALTH_POLL_MS
    ),
  };
}

export function getRuntimeServerBaseUrl(): string {
  return getRuntimeConfig().apiUrl;
}

export function getRuntimeApiBaseUrl(): string {
  const serverBaseUrl = getRuntimeServerBaseUrl();
  return serverBaseUrl ? `${serverBaseUrl}/api/v1` : '/api/v1';
}
