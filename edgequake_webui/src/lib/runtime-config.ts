export interface EdgeQuakeRuntimeConfig {
  apiUrl: string;
  authEnabled: boolean;
  disableDemoLogin: boolean;
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
    apiUrl: (
      browserConfig?.apiUrl ??
      process.env.EDGEQUAKE_API_URL ??
      process.env.NEXT_PUBLIC_API_URL ??
      ''
    ).replace(/\/$/, ''),
    authEnabled: parseBoolean(browserConfig?.authEnabled ?? process.env.NEXT_PUBLIC_AUTH_ENABLED),
    disableDemoLogin: parseBoolean(
      browserConfig?.disableDemoLogin ?? process.env.NEXT_PUBLIC_DISABLE_DEMO_LOGIN
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
