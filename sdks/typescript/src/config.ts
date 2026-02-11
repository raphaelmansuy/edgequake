/**
 * Configuration types and resolver.
 *
 * WHY: Resolves config from constructor args + environment variables,
 * providing sensible defaults for development and production.
 *
 * @module config
 */

/**
 * EdgeQuake SDK configuration.
 *
 * All fields are optional — defaults are resolved from environment
 * variables or sensible fallbacks.
 */
export interface EdgeQuakeConfig {
  /** Base URL of the EdgeQuake API server. Default: http://localhost:8080 */
  baseUrl?: string;

  /** API key for authentication (recommended for services). */
  apiKey?: string;

  /** JWT access token (manual token management). */
  accessToken?: string;

  /** Username/password credentials (auto-login on first request). */
  credentials?: {
    username: string;
    password: string;
  };

  /** Tenant ID for multi-tenant isolation. */
  tenantId?: string;

  /** User ID for user-scoped operations (required for chat endpoints). */
  userId?: string;

  /** Workspace ID for workspace-scoped operations. */
  workspaceId?: string;

  /** Request timeout in milliseconds. Default: 30000 */
  timeout?: number;

  /** Maximum retry attempts for retryable errors (429, 503). Default: 3 */
  maxRetries?: number;

  /** Custom fetch implementation (for testing, polyfills, or proxying). */
  fetch?: typeof fetch;

  /**
   * Inject a custom transport (testing / advanced usage).
   *
   * WHY: Allows unit tests to inject a mock transport without HTTP I/O.
   * When set, baseUrl/apiKey/fetch are ignored — the transport handles everything.
   * @internal
   */
  _transport?: import("./transport/types.js").HttpTransport;
}

/** Fully resolved configuration with no optional fields. */
export interface ResolvedConfig {
  baseUrl: string;
  apiKey: string;
  accessToken: string;
  credentials?: { username: string; password: string };
  tenantId: string;
  userId: string;
  workspaceId: string;
  timeout: number;
  maxRetries: number;
  fetchFn: typeof fetch;
}

/**
 * Resolve configuration from user options + environment variables.
 *
 * WHY: Reads EDGEQUAKE_* env vars so SDK works without explicit config
 * in deployed environments (Docker, K8s, CI/CD).
 */
export function resolveConfig(config?: EdgeQuakeConfig): ResolvedConfig {
  // WHY: Safe env access works in Node, Deno, Bun, and browser (undefined)
  const env =
    typeof process !== "undefined"
      ? process.env
      : ({} as Record<string, string | undefined>);

  return {
    baseUrl:
      config?.baseUrl ?? env.EDGEQUAKE_BASE_URL ?? "http://localhost:8080",
    apiKey: config?.apiKey ?? env.EDGEQUAKE_API_KEY ?? "",
    accessToken: config?.accessToken ?? "",
    credentials: config?.credentials,
    tenantId: config?.tenantId ?? env.EDGEQUAKE_TENANT_ID ?? "",
    userId: config?.userId ?? env.EDGEQUAKE_USER_ID ?? "",
    workspaceId: config?.workspaceId ?? env.EDGEQUAKE_WORKSPACE_ID ?? "",
    timeout: config?.timeout ?? 30_000,
    maxRetries: config?.maxRetries ?? 3,
    fetchFn: config?.fetch ?? globalThis.fetch,
  };
}
