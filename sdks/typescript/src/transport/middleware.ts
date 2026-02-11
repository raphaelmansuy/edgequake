/**
 * Authentication and tenant middleware.
 *
 * WHY: Middleware pattern keeps auth logic out of the transport core.
 * Each middleware adds headers non-destructively, supporting stacking.
 *
 * @module transport/middleware
 */

import type { Middleware, RequestOptions } from "./types.js";

/** Auth configuration for middleware. */
export interface AuthConfig {
  apiKey?: string;
  accessToken?: string;
}

/** Tenant configuration for middleware. */
export interface TenantConfig {
  tenantId?: string;
  userId?: string;
  workspaceId?: string;
}

/**
 * Creates auth middleware that adds API key or Bearer token headers.
 *
 * WHY: API key is preferred for service-to-service auth,
 * Bearer token for user-facing applications with JWT.
 */
export function createAuthMiddleware(auth: AuthConfig): Middleware {
  return async (req: RequestOptions, next) => {
    const headers = { ...req.headers };
    if (auth.apiKey) {
      headers["X-API-Key"] = auth.apiKey;
    } else if (auth.accessToken) {
      headers["Authorization"] = `Bearer ${auth.accessToken}`;
    }
    return next({ ...req, headers });
  };
}

/**
 * Creates tenant middleware that adds multi-tenancy headers.
 *
 * WHY: Multi-tenant isolation is enforced server-side via headers.
 * This ensures every request carries the correct tenant/workspace context.
 */
export function createTenantMiddleware(tenant: TenantConfig): Middleware {
  return async (req: RequestOptions, next) => {
    const headers = { ...req.headers };
    if (tenant.tenantId) {
      headers["X-Tenant-ID"] = tenant.tenantId;
    }
    if (tenant.userId) {
      headers["X-User-ID"] = tenant.userId;
    }
    if (tenant.workspaceId) {
      headers["X-Workspace-ID"] = tenant.workspaceId;
    }
    return next({ ...req, headers });
  };
}
