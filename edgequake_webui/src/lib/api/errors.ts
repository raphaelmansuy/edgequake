/**
 * Transport error classes for the API client.
 *
 * Split out of `client.ts` (SPEC-017 god-module guard). These are pure data types with no
 * dependency on the request pipeline, so keeping them beside ~300 lines of fetch/timeout/logging
 * logic only made that module harder to read. Re-exported from `client.ts`, so every existing
 * `import { ApiRequestError } from "@/lib/api/client"` keeps working.
 */

import type { ApiError } from "@/types";

/** Any non-2xx response, carrying the status and the server's machine-readable code. */
export class ApiRequestError extends Error {
  constructor(
    message: string,
    public status: number,
    public code?: string,
    public details?: Record<string, unknown>,
  ) {
    super(message);
    this.name = "ApiRequestError";
  }

  static fromResponse(error: ApiError): ApiRequestError {
    return new ApiRequestError(error.message, error.status, error.code, error.details);
  }
}

/** 401: the session is gone and the caller must re-authenticate. */
export class AuthError extends ApiRequestError {
  constructor(message: string = "Authentication required") {
    super(message, 401, "AUTH_REQUIRED");
    this.name = "AuthError";
  }
}

/** The request never reached the server (offline, DNS, CORS, connection reset). */
export class NetworkError extends Error {
  constructor(message: string = "Network request failed") {
    super(message);
    this.name = "NetworkError";
  }
}

/** The client-side deadline elapsed before a response arrived. */
export class TimeoutError extends ApiRequestError {
  constructor(message = "Request timed out") {
    super(message, 504, "timeout");
    this.name = "TimeoutError";
  }
}
