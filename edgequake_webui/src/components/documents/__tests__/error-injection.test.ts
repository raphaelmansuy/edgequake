/**
 * @module error-injection.test
 * @description Error injection tests for PDF progress tracking
 *
 * @implements OODA-38: Error injection tests
 *
 * Tests cover:
 * - Network failure simulation
 * - Timeout scenarios
 * - Corrupt PDF handling
 * - Error recovery flows
 * - Graceful degradation
 */

import { describe, expect, it } from "vitest";

// ============================================================================
// Types (matching component types)
// ============================================================================

interface PdfError {
  code: string;
  message: string;
  phase?: string;
  page?: number;
  chunk?: number;
  details?: string;
  recoverable?: boolean;
  suggestions?: string[];
}

interface PhaseProgress {
  phase: string;
  status: "pending" | "active" | "complete" | "failed";
  current: number;
  total: number;
  percentage: number;
  error?: PdfError;
  message: string;
}

// ============================================================================
// Error Injection Utilities
// ============================================================================

/**
 * WHY: Creates a mock error with specific characteristics
 * Allows testing different error scenarios
 */
function createError(
  code: string,
  message: string,
  options: Partial<PdfError> = {},
): PdfError {
  return {
    code,
    message,
    recoverable: true,
    ...options,
  };
}

/**
 * WHY: Simulates a phase that fails at a specific point
 */
function createFailedPhase(
  phase: string,
  failAt: number,
  total: number,
  error: PdfError,
): PhaseProgress {
  return {
    phase,
    status: "failed",
    current: failAt,
    total,
    percentage: Math.round((failAt / total) * 100),
    error,
    message: `Failed at ${failAt}/${total}: ${error.message}`,
  };
}

/**
 * WHY: Determines if an error is retryable based on type
 */
function isRetryable(error: PdfError): boolean {
  const code = error.code.toLowerCase();

  // Network errors are usually retryable
  if (code.includes("network") || code.includes("timeout")) {
    return true;
  }

  // Rate limits are retryable after delay
  if (code.includes("rate_limit") || code.includes("429")) {
    return true;
  }

  // LLM errors may be retryable
  if (code.includes("llm") && error.recoverable !== false) {
    return true;
  }

  // Parse errors are not retryable (corrupt file)
  if (code.includes("parse") || code.includes("corrupt")) {
    return false;
  }

  // Storage errors depend on recoverability
  if (code.includes("storage") || code.includes("database")) {
    return error.recoverable ?? false;
  }

  return error.recoverable ?? false;
}

/**
 * WHY: Calculates retry delay based on error type
 */
function calculateRetryDelay(error: PdfError, attempt: number): number {
  const code = error.code.toLowerCase();
  const baseDelay = 1000; // 1 second

  // Rate limits need longer delays
  if (code.includes("rate_limit") || code.includes("429")) {
    return 30000 + attempt * 5000; // 30s + 5s per attempt
  }

  // Timeout errors - exponential backoff
  if (code.includes("timeout")) {
    return Math.min(baseDelay * Math.pow(2, attempt), 60000); // Max 60s
  }

  // Default: linear backoff
  return baseDelay * (attempt + 1);
}

/**
 * WHY: Determines maximum retry attempts based on error type
 */
function getMaxRetries(error: PdfError): number {
  const code = error.code.toLowerCase();

  if (code.includes("rate_limit")) return 5;
  if (code.includes("timeout") || code.includes("network")) return 3;
  if (code.includes("llm")) return 2;

  return 1; // Default single retry
}

// ============================================================================
// Tests
// ============================================================================

describe("Error Creation", () => {
  it("creates basic error with required fields", () => {
    const error = createError("timeout", "Request timed out");

    expect(error.code).toBe("timeout");
    expect(error.message).toBe("Request timed out");
    expect(error.recoverable).toBe(true); // Default
  });

  it("creates error with all optional fields", () => {
    const error = createError("parse_error", "Invalid PDF structure", {
      phase: "PdfConversion",
      page: 5,
      details: "Missing EOF marker",
      recoverable: false,
      suggestions: ["Try re-exporting the PDF"],
    });

    expect(error.phase).toBe("PdfConversion");
    expect(error.page).toBe(5);
    expect(error.recoverable).toBe(false);
    expect(error.suggestions).toHaveLength(1);
  });
});

describe("Failed Phase Simulation", () => {
  it("creates failed phase at specific point", () => {
    const error = createError("timeout", "Connection lost");
    const phase = createFailedPhase("PdfConversion", 5, 10, error);

    expect(phase.status).toBe("failed");
    expect(phase.current).toBe(5);
    expect(phase.total).toBe(10);
    expect(phase.percentage).toBe(50);
    expect(phase.error).toBeDefined();
  });

  it("includes error in message", () => {
    const error = createError("network_error", "Server unreachable");
    const phase = createFailedPhase("Embedding", 3, 20, error);

    expect(phase.message).toContain("3/20");
    expect(phase.message).toContain("Server unreachable");
  });
});

describe("Error Retryability", () => {
  describe("retryable errors", () => {
    it("network errors are retryable", () => {
      const error = createError("network_error", "Connection reset");
      expect(isRetryable(error)).toBe(true);
    });

    it("timeout errors are retryable", () => {
      const error = createError("request_timeout", "Timed out after 30s");
      expect(isRetryable(error)).toBe(true);
    });

    it("rate limit errors are retryable", () => {
      const error = createError("rate_limit_exceeded", "Too many requests");
      expect(isRetryable(error)).toBe(true);
    });

    it("429 status is retryable", () => {
      const error = createError("http_429", "Rate limit");
      expect(isRetryable(error)).toBe(true);
    });

    it("LLM errors are retryable by default", () => {
      const error = createError("llm_error", "API error");
      expect(isRetryable(error)).toBe(true);
    });
  });

  describe("non-retryable errors", () => {
    it("parse errors are not retryable", () => {
      const error = createError("parse_error", "Invalid structure");
      expect(isRetryable(error)).toBe(false);
    });

    it("corrupt file errors are not retryable", () => {
      const error = createError("corrupt_file", "File corrupted");
      expect(isRetryable(error)).toBe(false);
    });

    it("LLM errors with recoverable=false are not retryable", () => {
      const error = createError("llm_error", "Invalid input", {
        recoverable: false,
      });
      expect(isRetryable(error)).toBe(false);
    });

    it("storage errors are not retryable by default", () => {
      const error = createError("storage_error", "Disk full", {
        recoverable: false,
      });
      expect(isRetryable(error)).toBe(false);
    });
  });

  describe("edge cases", () => {
    it("handles case-insensitive error codes", () => {
      const error = createError("NETWORK_ERROR", "Connection lost");
      expect(isRetryable(error)).toBe(true);
    });

    it("respects explicit recoverable flag", () => {
      const error = createError("unknown", "Something went wrong", {
        recoverable: true,
      });
      expect(isRetryable(error)).toBe(true);
    });
  });
});

describe("Retry Delay Calculation", () => {
  describe("rate limit delays", () => {
    it("starts with 30s delay", () => {
      const error = createError("rate_limit", "Rate limited");
      expect(calculateRetryDelay(error, 0)).toBe(30000);
    });

    it("increases by 5s per attempt", () => {
      const error = createError("rate_limit", "Rate limited");
      expect(calculateRetryDelay(error, 1)).toBe(35000);
      expect(calculateRetryDelay(error, 2)).toBe(40000);
    });
  });

  describe("timeout delays (exponential backoff)", () => {
    it("starts with 1s delay", () => {
      const error = createError("timeout", "Timed out");
      expect(calculateRetryDelay(error, 0)).toBe(1000);
    });

    it("doubles each attempt", () => {
      const error = createError("timeout", "Timed out");
      expect(calculateRetryDelay(error, 1)).toBe(2000);
      expect(calculateRetryDelay(error, 2)).toBe(4000);
      expect(calculateRetryDelay(error, 3)).toBe(8000);
    });

    it("caps at 60 seconds", () => {
      const error = createError("timeout", "Timed out");
      expect(calculateRetryDelay(error, 10)).toBe(60000);
    });
  });

  describe("default delays (linear)", () => {
    it("increases linearly", () => {
      const error = createError("unknown", "Error");
      expect(calculateRetryDelay(error, 0)).toBe(1000);
      expect(calculateRetryDelay(error, 1)).toBe(2000);
      expect(calculateRetryDelay(error, 2)).toBe(3000);
    });
  });
});

describe("Maximum Retry Attempts", () => {
  it("allows 5 retries for rate limits", () => {
    const error = createError("rate_limit", "Too many requests");
    expect(getMaxRetries(error)).toBe(5);
  });

  it("allows 3 retries for network errors", () => {
    const error = createError("network_error", "Connection failed");
    expect(getMaxRetries(error)).toBe(3);
  });

  it("allows 3 retries for timeout", () => {
    const error = createError("timeout", "Request timeout");
    expect(getMaxRetries(error)).toBe(3);
  });

  it("allows 2 retries for LLM errors", () => {
    const error = createError("llm_error", "API failed");
    expect(getMaxRetries(error)).toBe(2);
  });

  it("allows 1 retry by default", () => {
    const error = createError("unknown", "Something wrong");
    expect(getMaxRetries(error)).toBe(1);
  });
});

describe("Error Scenarios", () => {
  describe("Network Failure Simulation", () => {
    it("handles connection refused", () => {
      const error = createError("network_error", "Connection refused", {
        details: "ECONNREFUSED: localhost:3000",
      });

      expect(isRetryable(error)).toBe(true);
      expect(getMaxRetries(error)).toBe(3);
    });

    it("handles DNS resolution failure", () => {
      const error = createError("network_error", "DNS lookup failed", {
        details: "ENOTFOUND: api.example.com",
        recoverable: true,
      });

      expect(isRetryable(error)).toBe(true);
    });
  });

  describe("Timeout Scenarios", () => {
    it("handles request timeout at upload phase", () => {
      const error = createError("timeout", "Upload timed out", {
        phase: "Upload",
        recoverable: true,
      });
      const phase = createFailedPhase("Upload", 50, 100, error);

      expect(phase.percentage).toBe(50);
      expect(isRetryable(error)).toBe(true);
    });

    it("handles processing timeout", () => {
      const error = createError("timeout", "Processing exceeded 5 minutes", {
        phase: "Extraction",
        chunk: 45,
      });

      expect(calculateRetryDelay(error, 0)).toBe(1000);
    });
  });

  describe("Corrupt PDF Handling", () => {
    it("handles missing EOF marker", () => {
      const error = createError("parse_error", "Missing EOF marker", {
        phase: "PdfConversion",
        recoverable: false,
        suggestions: [
          "Re-export the PDF from source application",
          "Try a different PDF viewer to validate",
        ],
      });

      expect(isRetryable(error)).toBe(false);
      expect(error.suggestions).toHaveLength(2);
    });

    it("handles encrypted PDF", () => {
      const error = createError("parse_error", "PDF is password protected", {
        phase: "PdfConversion",
        recoverable: false,
        suggestions: ["Remove password protection before uploading"],
      });

      expect(isRetryable(error)).toBe(false);
    });

    it("handles page extraction failure", () => {
      const error = createError("parse_error", "Cannot extract page 5", {
        phase: "PdfConversion",
        page: 5,
        details: "Invalid page object reference",
      });
      const phase = createFailedPhase("PdfConversion", 5, 20, error);

      expect(phase.current).toBe(5);
      expect(error.page).toBe(5);
    });
  });

  describe("LLM Error Handling", () => {
    it("handles API rate limit", () => {
      const error = createError("llm_rate_limit", "Rate limit exceeded", {
        phase: "Extraction",
        recoverable: true,
      });

      expect(isRetryable(error)).toBe(true);
    });

    it("handles context length exceeded", () => {
      const error = createError("llm_error", "Context length exceeded", {
        phase: "Extraction",
        chunk: 3,
        details: "Input: 150000 tokens, Max: 128000 tokens",
        recoverable: false,
        suggestions: ["Chunk size is too large - try smaller chunks"],
      });

      expect(isRetryable(error)).toBe(false);
    });

    it("handles API unavailable", () => {
      const error = createError("llm_error", "OpenAI API unavailable", {
        phase: "Extraction",
        recoverable: true,
      });

      expect(isRetryable(error)).toBe(true);
      expect(getMaxRetries(error)).toBe(2);
    });
  });

  describe("Storage Error Handling", () => {
    it("handles database connection lost", () => {
      const error = createError("database_error", "Connection lost", {
        phase: "GraphStorage",
        recoverable: true,
      });

      expect(isRetryable(error)).toBe(true);
    });

    it("handles disk full", () => {
      const error = createError("storage_error", "No space left on device", {
        phase: "GraphStorage",
        recoverable: false,
      });

      expect(isRetryable(error)).toBe(false);
    });

    it("handles unique constraint violation", () => {
      const error = createError("database_error", "Duplicate key", {
        phase: "GraphStorage",
        details: "Entity already exists: JOHN_SMITH",
        recoverable: false,
      });

      expect(isRetryable(error)).toBe(false);
    });
  });
});

describe("Error Recovery Flow", () => {
  interface RetryState {
    attempts: number;
    lastError: PdfError | null;
    totalDelay: number;
    canRetry: boolean;
  }

  function simulateRetry(error: PdfError): RetryState {
    if (!isRetryable(error)) {
      return {
        attempts: 0,
        lastError: error,
        totalDelay: 0,
        canRetry: false,
      };
    }

    const maxRetries = getMaxRetries(error);
    let totalDelay = 0;

    for (let i = 0; i < maxRetries; i++) {
      totalDelay += calculateRetryDelay(error, i);
    }

    return {
      attempts: maxRetries,
      lastError: error,
      totalDelay,
      canRetry: true,
    };
  }

  it("simulates full retry cycle for network error", () => {
    const error = createError("network_error", "Connection failed");
    const state = simulateRetry(error);

    expect(state.attempts).toBe(3);
    expect(state.canRetry).toBe(true);
    // 1s + 2s + 3s = 6s total
    expect(state.totalDelay).toBe(6000);
  });

  it("simulates rate limit retry with backoff", () => {
    const error = createError("rate_limit", "Too many requests");
    const state = simulateRetry(error);

    expect(state.attempts).toBe(5);
    // 30s + 35s + 40s + 45s + 50s = 200s
    expect(state.totalDelay).toBe(200000);
  });

  it("does not retry non-recoverable errors", () => {
    const error = createError("parse_error", "Corrupt file");
    const state = simulateRetry(error);

    expect(state.attempts).toBe(0);
    expect(state.canRetry).toBe(false);
  });
});
