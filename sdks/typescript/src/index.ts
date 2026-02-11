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
export {
  resolveConfig,
  type EdgeQuakeConfig,
  type ResolvedConfig,
} from "./config.js";

// Errors
export {
  BadRequestError,
  ConflictError,
  EdgeQuakeError,
  ForbiddenError,
  InternalError,
  NetworkError,
  NotFoundError,
  PayloadTooLargeError,
  RateLimitError,
  ServiceUnavailableError,
  TimeoutError,
  UnauthorizedError,
  ValidationError,
  parseErrorResponse,
} from "./errors.js";

// Pagination
export { Paginator } from "./pagination.js";

// Streaming
export { parseSSEStream } from "./streaming/sse.js";
export { EdgeQuakeWebSocket } from "./streaming/websocket.js";

// Transport (advanced usage)
export { FetchTransport, createTransport } from "./transport/index.js";
export type {
  HttpTransport,
  Middleware,
  RequestOptions,
  TransportConfig,
} from "./transport/types.js";

// Types — re-export all types for consumers
export type * from "./types/index.js";
