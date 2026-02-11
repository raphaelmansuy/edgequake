/**
 * @edgequake/sdk — TypeScript SDK for the EdgeQuake RAG API.
 *
 * @example
 * ```ts
 * import { EdgeQuake } from "@edgequake/sdk";
 *
 * const client = new EdgeQuake({ apiKey: "eq-key-xxx" });
 * const health = await client.health();
 * ```
 *
 * @module @edgequake/sdk
 */

// Client
export { EdgeQuake } from "./client.js";

// Config
export { resolveConfig, type EdgeQuakeConfig, type ResolvedConfig } from "./config.js";

// Errors
export {
  EdgeQuakeError,
  BadRequestError,
  UnauthorizedError,
  ForbiddenError,
  NotFoundError,
  ConflictError,
  PayloadTooLargeError,
  ValidationError,
  RateLimitError,
  InternalError,
  ServiceUnavailableError,
  TimeoutError,
  NetworkError,
  parseErrorResponse,
} from "./errors.js";

// Pagination
export { Paginator } from "./pagination.js";

// Streaming
export { parseSSEStream } from "./streaming/sse.js";
export { EdgeQuakeWebSocket } from "./streaming/websocket.js";

// Transport (advanced usage)
export { createTransport, FetchTransport } from "./transport/index.js";
export type { HttpTransport, RequestOptions, TransportConfig, Middleware } from "./transport/types.js";

// Types — re-export all types for consumers
export type * from "./types/index.js";
