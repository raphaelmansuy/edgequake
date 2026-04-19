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

  return {
    apiUrl: (browserConfig?.apiUrl ?? process.env.NEXT_PUBLIC_API_URL ?? '').replace(/\/$/, ''),
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
