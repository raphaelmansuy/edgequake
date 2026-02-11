/**
 * EdgeQuake SDK Error Classes
 *
 * Typed error hierarchy matching the EdgeQuake API error responses.
 * Each HTTP status code maps to a specific error class for precise catch handling.
 *
 * @module errors
 * @see edgequake/crates/edgequake-api/src/error.rs
 */

// ── Base Error ────────────────────────────────────────────────

/**
 * Base error class for all EdgeQuake API errors.
 *
 * WHY: A typed error hierarchy lets consumers use `instanceof` checks
 * for precise error handling instead of parsing status codes manually.
 */
export class EdgeQuakeError extends Error {
  /** Machine-readable error code (e.g., "NOT_FOUND", "RATE_LIMITED"). */
  readonly code: string;
  /** HTTP status code from the response (0 for network errors). */
  readonly status: number;
  /** Optional structured details from the API response. */
  readonly details?: Record<string, unknown>;

  constructor(
    message: string,
    code: string,
    status: number,
    details?: Record<string, unknown>,
  ) {
    super(message);
    this.name = "EdgeQuakeError";
    this.code = code;
    this.status = status;
    this.details = details;
    // WHY: Ensure proper prototype chain for instanceof checks in transpiled code
    Object.setPrototypeOf(this, new.target.prototype);
  }

  /** Whether the error is retryable (429, 503). */
  get isRetryable(): boolean {
    return this.status === 429 || this.status === 503;
  }

  /** Whether this is a client-side error (4xx). */
  get isClient(): boolean {
    return this.status >= 400 && this.status < 500;
  }

  /** Whether this is a server-side error (5xx). */
  get isServer(): boolean {
    return this.status >= 500;
  }
}

// ── Specific Error Classes ────────────────────────────────────

/** 400 Bad Request — invalid request parameters. */
export class BadRequestError extends EdgeQuakeError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(message, "BAD_REQUEST", 400, details);
    this.name = "BadRequestError";
  }
}

/** 401 Unauthorized — missing or invalid credentials. */
export class UnauthorizedError extends EdgeQuakeError {
  constructor(message: string) {
    super(message, "UNAUTHORIZED", 401);
    this.name = "UnauthorizedError";
  }
}

/** 403 Forbidden — insufficient permissions. */
export class ForbiddenError extends EdgeQuakeError {
  constructor(message: string) {
    super(message, "FORBIDDEN", 403);
    this.name = "ForbiddenError";
  }
}

/** 404 Not Found — resource does not exist. */
export class NotFoundError extends EdgeQuakeError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(message, "NOT_FOUND", 404, details);
    this.name = "NotFoundError";
  }
}

/** 409 Conflict — resource already exists or conflicting state. */
export class ConflictError extends EdgeQuakeError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(message, "CONFLICT", 409, details);
    this.name = "ConflictError";
  }
}

/** 413 Payload Too Large — request body exceeds size limit. */
export class PayloadTooLargeError extends EdgeQuakeError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(message, "PAYLOAD_TOO_LARGE", 413, details);
    this.name = "PayloadTooLargeError";
  }
}

/** 422 Validation Error — request body failed validation. */
export class ValidationError extends EdgeQuakeError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(message, "VALIDATION_ERROR", 422, details);
    this.name = "ValidationError";
  }
}

/** 429 Rate Limited — too many requests. */
export class RateLimitError extends EdgeQuakeError {
  /** Seconds to wait before retrying (from Retry-After header). */
  readonly retryAfter?: number;

  constructor(message: string, retryAfter?: number) {
    super(message, "RATE_LIMITED", 429);
    this.name = "RateLimitError";
    this.retryAfter = retryAfter;
  }
}

/** 500 Internal Server Error — unexpected server failure. */
export class InternalError extends EdgeQuakeError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(message, "INTERNAL_ERROR", 500, details);
    this.name = "InternalError";
  }
}

/** 503 Service Unavailable — server temporarily unable to handle request. */
export class ServiceUnavailableError extends EdgeQuakeError {
  constructor(message: string) {
    super(message, "SERVICE_UNAVAILABLE", 503);
    this.name = "ServiceUnavailableError";
  }
}

/** 408 Timeout — request timed out. */
export class TimeoutError extends EdgeQuakeError {
  constructor(message: string) {
    super(message, "TIMEOUT", 408);
    this.name = "TimeoutError";
  }
}

/** Network Error — connection failed (status 0). */
export class NetworkError extends EdgeQuakeError {
  constructor(message: string, cause?: Error) {
    super(message, "NETWORK_ERROR", 0);
    this.name = "NetworkError";
    if (cause) {
      this.cause = cause;
    }
  }
}

// ── Error Response Body ──────────────────────────────────────

/** Shape of the JSON error response from the EdgeQuake API. */
export interface ErrorResponseBody {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

/**
 * Parse an HTTP error response into a typed EdgeQuakeError.
 *
 * WHY: Centralizes error mapping so consumers always get typed errors
 * regardless of which resource method threw them.
 */
export function parseErrorResponse(
  status: number,
  body: ErrorResponseBody,
): EdgeQuakeError {
  const { code, message, details } = body;

  switch (status) {
    case 400:
      return new BadRequestError(message, details);
    case 401:
      return new UnauthorizedError(message);
    case 403:
      return new ForbiddenError(message);
    case 404:
      return new NotFoundError(message, details);
    case 409:
      return new ConflictError(message, details);
    case 413:
      return new PayloadTooLargeError(message, details);
    case 422:
      return new ValidationError(message, details);
    case 429:
      return new RateLimitError(message);
    case 500:
      return new InternalError(message, details);
    case 503:
      return new ServiceUnavailableError(message);
    default:
      return new EdgeQuakeError(message, code, status, details);
  }
}
