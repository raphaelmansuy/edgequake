/**
 * Transport factory.
 *
 * WHY: Centralizes transport creation with proper middleware ordering.
 * Consumers only provide config — middleware stack is assembled automatically.
 *
 * @module transport
 */

export { FetchTransport } from "./fetch.js";
export { createAuthMiddleware, createTenantMiddleware } from "./middleware.js";
export type { AuthConfig, TenantConfig } from "./middleware.js";
export { createRetryMiddleware } from "./retry.js";
export type { RetryConfig } from "./retry.js";
export type {
  HttpTransport,
  Middleware,
  RequestOptions,
  TransportConfig,
} from "./types.js";

import type { ResolvedConfig } from "../config.js";
import { FetchTransport } from "./fetch.js";
import { createAuthMiddleware, createTenantMiddleware } from "./middleware.js";
import { createRetryMiddleware } from "./retry.js";
import type { HttpTransport, TransportConfig } from "./types.js";

/**
 * Create a transport from a resolved SDK config.
 *
 * Middleware order: auth → tenant → retry → fetch
 * WHY: Auth runs first so retry middleware can re-send with credentials.
 * Tenant headers are static per-client so they run before retry.
 */
export function createTransport(config: ResolvedConfig): HttpTransport {
  const transportConfig: TransportConfig = {
    baseUrl: config.baseUrl,
    headers: {},
    timeout: config.timeout,
    maxRetries: config.maxRetries,
    retryDelay: 1000,
    retryStatusCodes: [429, 502, 503, 504],
    fetchFn: config.fetchFn,
  };

  const middlewares = [
    createAuthMiddleware({
      apiKey: config.apiKey,
      accessToken: config.accessToken,
    }),
    createTenantMiddleware({
      tenantId: config.tenantId,
      userId: config.userId,
      workspaceId: config.workspaceId,
    }),
    createRetryMiddleware({
      maxRetries: config.maxRetries,
      retryDelay: 1000,
      retryStatusCodes: [429, 502, 503, 504],
    }),
  ];

  return new FetchTransport(transportConfig, middlewares);
}
