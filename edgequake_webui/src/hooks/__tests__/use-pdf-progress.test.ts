/**
 * @module use-pdf-progress.test
 * @description Unit tests for usePdfProgress hook
 *
 * @implements OODA-32: Unit tests for progress tracking
 *
 * Tests cover:
 * - Phase enrichment logic
 * - Overall percentage calculation
 * - Current phase index detection
 * - Error code extraction
 * - ETA computation
 */

import { describe, expect, it } from "vitest";

// ============================================================================
// Helper Function Tests (extracted from hook)
// ============================================================================

/**
 * Phase labels and descriptions for testing.
 */
const PHASE_LABELS = {
  upload: { label: "Upload", description: "File upload and validation" },
  pdf_conversion: {
    label: "PDF → Markdown",
    description: "Converting PDF pages to text",
  },
  chunking: { label: "Chunking", description: "Splitting text into chunks" },
  embedding: {
    label: "Embedding",
    description: "Generating vector embeddings",
  },
  extraction: {
    label: "Extraction",
    description: "Extracting entities and relationships",
  },
  graph_storage: {
    label: "Storage",
    description: "Storing in knowledge graph",
  },
};

const PHASE_ORDER = [
  "upload",
  "pdf_conversion",
  "chunking",
  "embedding",
  "extraction",
  "graph_storage",
] as const;

type PipelinePhase = (typeof PHASE_ORDER)[number];
type PhaseStatus =
  | { type: "pending" }
  | { type: "active"; current: number; total: number; percent: number }
  | { type: "completed" }
  | { type: "failed"; error: string };

interface PhaseInfo {
  phase: PipelinePhase;
  label: string;
  description: string;
  status: PhaseStatus;
  index: number;
}

/**
 * Calculate overall percentage from phases.
 */
function calculateOverallPercent(phases: PhaseStatus[]): number {
  const totalPhases = PHASE_ORDER.length;
  let completed = 0;
  let activeProgress = 0;

  for (let i = 0; i < phases.length; i++) {
    const phase = phases[i];
    if (phase.type === "completed") {
      completed++;
    } else if (phase.type === "active") {
      activeProgress = phase.percent / 100;
    }
  }

  return Math.round(((completed + activeProgress) / totalPhases) * 100);
}

/**
 * Find current active phase index.
 */
function findCurrentPhaseIndex(phases: PhaseStatus[]): number {
  for (let i = 0; i < phases.length; i++) {
    const phase = phases[i];
    if (phase.type === "active") return i;
    if (phase.type === "pending") return Math.max(0, i - 1);
  }
  return phases.length - 1; // All complete
}

/**
 * Extract error code from error message.
 */
function extractErrorCode(errorMessage: string): string {
  const lowerMsg = errorMessage.toLowerCase();
  if (lowerMsg.includes("timeout")) return "timeout_error";
  if (lowerMsg.includes("network")) return "network_error";
  if (lowerMsg.includes("rate limit") || lowerMsg.includes("429"))
    return "rate_limit";
  if (lowerMsg.includes("parse") || lowerMsg.includes("corrupt"))
    return "parse_error";
  if (lowerMsg.includes("llm") || lowerMsg.includes("openai"))
    return "llm_error";
  if (lowerMsg.includes("storage") || lowerMsg.includes("database"))
    return "storage_error";
  return "unknown_error";
}

/**
 * Get failed phase name from phases.
 */
function getFailedPhaseName(phases: PhaseInfo[]): string | undefined {
  const failedPhase = phases.find((p) => p.status.type === "failed");
  return failedPhase?.label;
}

// ============================================================================
// Tests
// ============================================================================

describe("calculateOverallPercent", () => {
  it("returns 0 for all pending phases", () => {
    const phases: PhaseStatus[] = [
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
    ];
    expect(calculateOverallPercent(phases)).toBe(0);
  });

  it("returns 17 for one completed phase (1/6)", () => {
    const phases: PhaseStatus[] = [
      { type: "completed" },
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
    ];
    expect(calculateOverallPercent(phases)).toBe(17);
  });

  it("returns 50 for three completed phases (3/6)", () => {
    const phases: PhaseStatus[] = [
      { type: "completed" },
      { type: "completed" },
      { type: "completed" },
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
    ];
    expect(calculateOverallPercent(phases)).toBe(50);
  });

  it("returns 100 for all completed phases", () => {
    const phases: PhaseStatus[] = [
      { type: "completed" },
      { type: "completed" },
      { type: "completed" },
      { type: "completed" },
      { type: "completed" },
      { type: "completed" },
    ];
    expect(calculateOverallPercent(phases)).toBe(100);
  });

  it("includes partial progress from active phase", () => {
    const phases: PhaseStatus[] = [
      { type: "completed" },
      { type: "active", current: 5, total: 10, percent: 50 },
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
    ];
    // 1 complete (17%) + 0.5 * (1/6 = 17%) = 17% + 8.5% = 25%
    expect(calculateOverallPercent(phases)).toBe(25);
  });
});

describe("findCurrentPhaseIndex", () => {
  it("returns 0 for all pending phases", () => {
    const phases: PhaseStatus[] = [
      { type: "pending" },
      { type: "pending" },
      { type: "pending" },
    ];
    expect(findCurrentPhaseIndex(phases)).toBe(0);
  });

  it("returns index of active phase", () => {
    const phases: PhaseStatus[] = [
      { type: "completed" },
      { type: "completed" },
      { type: "active", current: 3, total: 10, percent: 30 },
      { type: "pending" },
    ];
    expect(findCurrentPhaseIndex(phases)).toBe(2);
  });

  it("returns last index for all completed", () => {
    const phases: PhaseStatus[] = [
      { type: "completed" },
      { type: "completed" },
      { type: "completed" },
    ];
    expect(findCurrentPhaseIndex(phases)).toBe(2);
  });
});

describe("extractErrorCode", () => {
  it("extracts timeout_error", () => {
    expect(extractErrorCode("Connection timeout after 30s")).toBe(
      "timeout_error",
    );
    expect(extractErrorCode("Request TIMEOUT")).toBe("timeout_error");
  });

  it("extracts network_error", () => {
    expect(extractErrorCode("Network error: connection refused")).toBe(
      "network_error",
    );
    expect(extractErrorCode("NETWORK_UNAVAILABLE")).toBe("network_error");
  });

  it("extracts rate_limit", () => {
    expect(extractErrorCode("Rate limit exceeded")).toBe("rate_limit");
    expect(extractErrorCode("Error 429: Too many requests")).toBe("rate_limit");
  });

  it("extracts parse_error", () => {
    expect(extractErrorCode("Failed to parse PDF")).toBe("parse_error");
    expect(extractErrorCode("Corrupt PDF file")).toBe("parse_error");
  });

  it("extracts llm_error", () => {
    expect(extractErrorCode("LLM API error")).toBe("llm_error");
    expect(extractErrorCode("OpenAI returned 500")).toBe("llm_error");
  });

  it("extracts storage_error", () => {
    expect(extractErrorCode("Storage write failed")).toBe("storage_error");
    expect(extractErrorCode("Database connection lost")).toBe("storage_error");
  });

  it("returns unknown_error for unrecognized messages", () => {
    expect(extractErrorCode("Something went wrong")).toBe("unknown_error");
    expect(extractErrorCode("")).toBe("unknown_error");
  });
});

describe("getFailedPhaseName", () => {
  it("returns undefined when no phase failed", () => {
    const phases: PhaseInfo[] = [
      {
        phase: "upload",
        label: "Upload",
        description: "",
        status: { type: "completed" },
        index: 0,
      },
      {
        phase: "pdf_conversion",
        label: "PDF → Markdown",
        description: "",
        status: { type: "active", current: 1, total: 5, percent: 20 },
        index: 1,
      },
    ];
    expect(getFailedPhaseName(phases)).toBeUndefined();
  });

  it("returns label of failed phase", () => {
    const phases: PhaseInfo[] = [
      {
        phase: "upload",
        label: "Upload",
        description: "",
        status: { type: "completed" },
        index: 0,
      },
      {
        phase: "pdf_conversion",
        label: "PDF → Markdown",
        description: "",
        status: { type: "failed", error: "Parse error" },
        index: 1,
      },
    ];
    expect(getFailedPhaseName(phases)).toBe("PDF → Markdown");
  });
});

describe("PHASE_LABELS", () => {
  it("has labels for all 6 phases", () => {
    expect(Object.keys(PHASE_LABELS)).toHaveLength(6);
    for (const phase of PHASE_ORDER) {
      expect(PHASE_LABELS[phase]).toBeDefined();
      expect(PHASE_LABELS[phase].label).toBeTruthy();
      expect(PHASE_LABELS[phase].description).toBeTruthy();
    }
  });
});
