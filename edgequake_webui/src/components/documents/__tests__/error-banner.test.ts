/**
 * @module error-banner.test
 * @description Unit tests for ErrorBanner component error classification
 *
 * @implements OODA-33: ErrorBanner error classification tests
 *
 * Tests cover:
 * - Error code classification
 * - Severity assignment
 * - Suggestion generation
 */

import { describe, expect, it } from "vitest";

// ============================================================================
// Types (matching error-banner.tsx)
// ============================================================================

type ErrorSeverity = "warning" | "error" | "critical";

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

// ============================================================================
// Classification Logic (extracted from component)
// ============================================================================

function classifyError(error: PdfError): {
  severity: ErrorSeverity;
  suggestions: string[];
} {
  const code = error.code?.toLowerCase() || "";

  // Network/timeout errors - usually recoverable
  if (code.includes("timeout") || code.includes("network")) {
    return {
      severity: "warning",
      suggestions: [
        "Check your internet connection",
        "The file might be too large - try splitting it",
        "Wait a moment and retry",
      ],
    };
  }

  // Rate limiting
  if (code.includes("rate_limit") || code.includes("429")) {
    return {
      severity: "warning",
      suggestions: [
        "Too many requests - please wait 30 seconds",
        "Consider uploading fewer files at once",
      ],
    };
  }

  // PDF parsing errors
  if (code.includes("parse") || code.includes("corrupt")) {
    return {
      severity: "error",
      suggestions: [
        "The PDF might be corrupted or password-protected",
        "Try re-exporting the PDF from the source application",
        "Convert to PDF/A format for better compatibility",
      ],
    };
  }

  // LLM/extraction errors
  if (code.includes("llm") || code.includes("extraction")) {
    return {
      severity: "warning",
      suggestions: [
        "The AI model encountered an issue processing this content",
        "Try with a smaller PDF or fewer pages",
        "Retry - this might be a temporary issue",
      ],
    };
  }

  // Storage/database errors
  if (code.includes("storage") || code.includes("database")) {
    return {
      severity: "critical",
      suggestions: [
        "There was an issue saving to the database",
        "Check server logs for more details",
        "Contact support if the issue persists",
      ],
    };
  }

  // Default fallback
  return {
    severity: "error",
    suggestions: error.suggestions || [
      "An unexpected error occurred",
      "Try retrying the upload",
      "Contact support if the issue persists",
    ],
  };
}

// ============================================================================
// Tests
// ============================================================================

describe("classifyError", () => {
  describe("timeout errors", () => {
    it("classifies timeout_error as warning", () => {
      const result = classifyError({
        code: "timeout_error",
        message: "Request timed out",
      });
      expect(result.severity).toBe("warning");
      expect(result.suggestions).toContain("Check your internet connection");
    });

    it("classifies network_timeout as warning", () => {
      const result = classifyError({
        code: "network_timeout",
        message: "Network unavailable",
      });
      expect(result.severity).toBe("warning");
    });
  });

  describe("rate limit errors", () => {
    it("classifies rate_limit as warning", () => {
      const result = classifyError({
        code: "rate_limit",
        message: "Too many requests",
      });
      expect(result.severity).toBe("warning");
      expect(result.suggestions).toContain(
        "Too many requests - please wait 30 seconds",
      );
    });

    it("classifies 429 errors as warning", () => {
      const result = classifyError({
        code: "http_429",
        message: "Rate limit exceeded",
      });
      expect(result.severity).toBe("warning");
    });
  });

  describe("parse errors", () => {
    it("classifies parse_error as error", () => {
      const result = classifyError({
        code: "parse_error",
        message: "Failed to parse PDF",
      });
      expect(result.severity).toBe("error");
      expect(result.suggestions).toContain(
        "The PDF might be corrupted or password-protected",
      );
    });

    it("classifies corrupt_file as error", () => {
      const result = classifyError({
        code: "corrupt_file",
        message: "File is corrupted",
      });
      expect(result.severity).toBe("error");
    });
  });

  describe("LLM errors", () => {
    it("classifies llm_error as warning", () => {
      const result = classifyError({
        code: "llm_error",
        message: "LLM API failed",
      });
      expect(result.severity).toBe("warning");
      expect(result.suggestions).toContain(
        "The AI model encountered an issue processing this content",
      );
    });

    it("classifies extraction_failed as warning", () => {
      const result = classifyError({
        code: "extraction_failed",
        message: "Entity extraction failed",
      });
      expect(result.severity).toBe("warning");
    });
  });

  describe("storage errors", () => {
    it("classifies storage_error as critical", () => {
      const result = classifyError({
        code: "storage_error",
        message: "Database write failed",
      });
      expect(result.severity).toBe("critical");
      expect(result.suggestions).toContain(
        "There was an issue saving to the database",
      );
    });

    it("classifies database_error as critical", () => {
      const result = classifyError({
        code: "database_connection_lost",
        message: "DB connection lost",
      });
      expect(result.severity).toBe("critical");
    });
  });

  describe("unknown errors", () => {
    it("classifies unknown errors as error", () => {
      const result = classifyError({
        code: "unknown",
        message: "Something went wrong",
      });
      expect(result.severity).toBe("error");
    });

    it("uses provided suggestions for unknown errors", () => {
      const customSuggestions = ["Custom suggestion 1", "Custom suggestion 2"];
      const result = classifyError({
        code: "custom_error",
        message: "Custom error",
        suggestions: customSuggestions,
      });
      expect(result.suggestions).toEqual(customSuggestions);
    });

    it("provides default suggestions when none provided", () => {
      const result = classifyError({
        code: "mystery",
        message: "Mystery error",
      });
      expect(result.suggestions).toContain("An unexpected error occurred");
    });
  });

  describe("case insensitivity", () => {
    it("handles uppercase error codes", () => {
      const result = classifyError({
        code: "TIMEOUT_ERROR",
        message: "Timeout",
      });
      expect(result.severity).toBe("warning");
    });

    it("handles mixed case error codes", () => {
      const result = classifyError({
        code: "Rate_Limit_Exceeded",
        message: "Rate limited",
      });
      expect(result.severity).toBe("warning");
    });
  });

  describe("empty/null handling", () => {
    it("handles empty error code", () => {
      const result = classifyError({ code: "", message: "Empty code error" });
      expect(result.severity).toBe("error");
    });

    it("handles undefined suggestions", () => {
      const result = classifyError({
        code: "unknown",
        message: "No suggestions",
      });
      expect(result.suggestions.length).toBeGreaterThan(0);
    });
  });
});

describe("PdfError interface", () => {
  it("accepts minimal error", () => {
    const error: PdfError = { code: "test", message: "Test error" };
    expect(error.code).toBe("test");
    expect(error.message).toBe("Test error");
  });

  it("accepts full error with all fields", () => {
    const error: PdfError = {
      code: "parse_error",
      message: "Failed to parse page 5",
      phase: "PdfConversion",
      page: 5,
      chunk: undefined,
      details: "Stack trace here...",
      recoverable: true,
      suggestions: ["Try again"],
    };
    expect(error.phase).toBe("PdfConversion");
    expect(error.page).toBe(5);
    expect(error.recoverable).toBe(true);
  });
});
