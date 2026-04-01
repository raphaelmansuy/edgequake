export interface EdgeQuakeRuntimeConfig {
  apiUrl: string;
  wsUrl: string;
}

declare global {
  // Generated at runtime by the container entrypoint or predev/prestart hook.
  var __EDGEQUAKE_RUNTIME_CONFIG__: EdgeQuakeRuntimeConfig | undefined;
}

const DEFAULT_API_URL = "http://localhost:8080";

const normalizeUrl = (value: string | undefined | null): string => {
  const trimmed = value?.trim();
  if (!trimmed) {
    return "";
  }

  return trimmed.replace(/\/$/, "");
};

const readEnvValue = (...names: string[]): string => {
  if (typeof process === "undefined" || !process.env) {
    return "";
  }

  for (const name of names) {
    const value = process.env[name];
    if (typeof value === "string") {
      const normalized = normalizeUrl(value);
      if (normalized) {
        return normalized;
      }
    }
  }

  return "";
};

const deriveWebSocketUrl = (apiUrl: string): string => {
  if (!apiUrl) {
    return "";
  }

  return apiUrl.replace(/^https:/, "wss:").replace(/^http:/, "ws:");
};

const readBrowserRuntimeConfig = (): Partial<EdgeQuakeRuntimeConfig> => {
  if (typeof globalThis === "undefined") {
    return {};
  }

  return globalThis.__EDGEQUAKE_RUNTIME_CONFIG__ ?? {};
};

/**
 * Resolve the runtime API URL.
 *
 * Priority:
 * 1. Runtime config injected into the browser
 * 2. Runtime environment variables on the server
 * 3. Safe development fallback
 */
export const getRuntimeApiUrl = (): string => {
  const browserConfig = readBrowserRuntimeConfig();
  const browserUrl = normalizeUrl(browserConfig.apiUrl);
  if (browserUrl) {
    return browserUrl;
  }

  const serverUrl = readEnvValue("EDGEQUAKE_API_URL", "NEXT_PUBLIC_API_URL");
  return serverUrl || DEFAULT_API_URL;
};

/**
 * Resolve the runtime WebSocket base URL.
 *
 * Priority:
 * 1. Runtime config injected into the browser
 * 2. Runtime environment variables on the server
 * 3. Derived from the API URL
 */
export const getRuntimeWebSocketUrl = (): string => {
  const browserConfig = readBrowserRuntimeConfig();
  const browserUrl = normalizeUrl(browserConfig.wsUrl);
  if (browserUrl) {
    return browserUrl;
  }

  const serverUrl = readEnvValue("EDGEQUAKE_WS_URL", "NEXT_PUBLIC_WS_URL");
  if (serverUrl) {
    return serverUrl;
  }

  return deriveWebSocketUrl(getRuntimeApiUrl());
};
