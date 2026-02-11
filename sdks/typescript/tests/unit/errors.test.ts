/**
 * Unit tests for error classes.
 *
 * @module tests/errors.test
 */

import { describe, expect, it } from "vitest";
import {
  BadRequestError,
  ConflictError,
  EdgeQuakeError,
  ForbiddenError,
  InternalError,
  NetworkError,
  NotFoundError,
  parseErrorResponse,
  PayloadTooLargeError,
  RateLimitError,
  ServiceUnavailableError,
  TimeoutError,
  UnauthorizedError,
  ValidationError,
} from "../../src/errors.js";

describe("EdgeQuakeError", () => {
  it("creates base error with message and status", () => {
    const error = new EdgeQuakeError("test error", "TEST_CODE", 500);
    expect(error.message).toBe("test error");
    expect(error.status).toBe(500);
    expect(error.code).toBe("TEST_CODE");
    expect(error).toBeInstanceOf(Error);
    expect(error).toBeInstanceOf(EdgeQuakeError);
  });

  it("preserves error name", () => {
    const error = new EdgeQuakeError("test", 500);
    expect(error.name).toBe("EdgeQuakeError");
  });
});

describe("HTTP error classes", () => {
  const errorCases: [
    string,
    new (m: string, c?: string) => EdgeQuakeError,
    number,
  ][] = [
    ["BadRequestError", BadRequestError, 400],
    ["UnauthorizedError", UnauthorizedError, 401],
    ["ForbiddenError", ForbiddenError, 403],
    ["NotFoundError", NotFoundError, 404],
    ["ConflictError", ConflictError, 409],
    ["PayloadTooLargeError", PayloadTooLargeError, 413],
    ["ValidationError", ValidationError, 422],
    ["RateLimitError", RateLimitError, 429],
    ["InternalError", InternalError, 500],
    ["ServiceUnavailableError", ServiceUnavailableError, 503],
    ["TimeoutError", TimeoutError, 408],
  ];

  for (const [name, ErrorClass, expectedStatus] of errorCases) {
    it(`${name} has status ${expectedStatus}`, () => {
      const error = new ErrorClass("test");
      expect(error.status).toBe(expectedStatus);
      expect(error.name).toBe(name);
      expect(error).toBeInstanceOf(EdgeQuakeError);
      expect(error).toBeInstanceOf(ErrorClass);
    });
  }

  it("NetworkError has status 0", () => {
    const error = new NetworkError("connection failed");
    expect(error.status).toBe(0);
    expect(error.name).toBe("NetworkError");
  });
});

describe("parseErrorResponse", () => {
  it("maps 400 → BadRequestError", () => {
    const error = parseErrorResponse(400, {
      code: "BAD_REQUEST",
      message: "bad",
    });
    expect(error).toBeInstanceOf(BadRequestError);
    expect(error.message).toBe("bad");
    expect(error.code).toBe("BAD_REQUEST");
  });

  it("maps 401 → UnauthorizedError", () => {
    const error = parseErrorResponse(401, { code: "UNAUTH", message: "nope" });
    expect(error).toBeInstanceOf(UnauthorizedError);
  });

  it("maps 403 → ForbiddenError", () => {
    const error = parseErrorResponse(403, {
      code: "FORBIDDEN",
      message: "denied",
    });
    expect(error).toBeInstanceOf(ForbiddenError);
  });

  it("maps 404 → NotFoundError", () => {
    const error = parseErrorResponse(404, {
      code: "NOT_FOUND",
      message: "missing",
    });
    expect(error).toBeInstanceOf(NotFoundError);
  });

  it("maps 409 → ConflictError", () => {
    const error = parseErrorResponse(409, {
      code: "CONFLICT",
      message: "exists",
    });
    expect(error).toBeInstanceOf(ConflictError);
  });

  it("maps 413 → PayloadTooLargeError", () => {
    const error = parseErrorResponse(413, {
      code: "TOO_LARGE",
      message: "big",
    });
    expect(error).toBeInstanceOf(PayloadTooLargeError);
  });

  it("maps 422 → ValidationError", () => {
    const error = parseErrorResponse(422, {
      code: "INVALID",
      message: "bad field",
    });
    expect(error).toBeInstanceOf(ValidationError);
  });

  it("maps 429 → RateLimitError", () => {
    const error = parseErrorResponse(429, {
      code: "RATE_LIMIT",
      message: "slow down",
    });
    expect(error).toBeInstanceOf(RateLimitError);
  });

  it("maps 500 → InternalError", () => {
    const error = parseErrorResponse(500, {
      code: "INTERNAL",
      message: "oops",
    });
    expect(error).toBeInstanceOf(InternalError);
  });

  it("maps 503 → ServiceUnavailableError", () => {
    const error = parseErrorResponse(503, { code: "DOWN", message: "offline" });
    expect(error).toBeInstanceOf(ServiceUnavailableError);
  });

  it("maps unknown status to EdgeQuakeError", () => {
    const error = parseErrorResponse(418, {
      code: "TEAPOT",
      message: "I'm a teapot",
    });
    expect(error).toBeInstanceOf(EdgeQuakeError);
    expect(error.status).toBe(418);
  });
});
