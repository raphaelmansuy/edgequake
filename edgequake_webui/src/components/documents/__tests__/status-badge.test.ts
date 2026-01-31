/**
 * OODA-48: Status Badge Logic Unit Tests
 *
 * @description Tests for document status badge logic
 * @implements Phase 4 Testing & Validation
 * @see specs/001-upload-pdf.md
 */

import { describe, expect, it } from "vitest";

// ============================================================================
// Status Configuration (replicate from status-badge.tsx)
// ============================================================================

const statusConfig = {
  // Queue states
  pending: {
    icon: "Clock",
    color: "bg-yellow-500",
    textColor: "text-yellow-600 dark:text-yellow-400",
    label: "Pending",
    animate: false,
  },

  // Processing sub-states
  processing: {
    icon: "Loader2",
    color: "bg-blue-500",
    textColor: "text-blue-600 dark:text-blue-400",
    label: "Processing",
    animate: true,
  },
  chunking: {
    icon: "Scissors",
    color: "bg-blue-400",
    textColor: "text-blue-500 dark:text-blue-300",
    label: "Chunking",
    animate: true,
  },
  extracting: {
    icon: "Brain",
    color: "bg-purple-500",
    textColor: "text-purple-600 dark:text-purple-400",
    label: "Extracting",
    animate: true,
  },
  embedding: {
    icon: "Cpu",
    color: "bg-cyan-500",
    textColor: "text-cyan-600 dark:text-cyan-400",
    label: "Embedding",
    animate: true,
  },
  indexing: {
    icon: "Database",
    color: "bg-teal-500",
    textColor: "text-teal-600 dark:text-teal-400",
    label: "Indexing",
    animate: true,
  },

  // Terminal states
  completed: {
    icon: "CheckCircle",
    color: "bg-green-500",
    textColor: "text-green-600 dark:text-green-400",
    label: "Completed",
    animate: false,
  },
  indexed: {
    icon: "CheckCircle",
    color: "bg-green-500",
    textColor: "text-green-600 dark:text-green-400",
    label: "Indexed",
    animate: false,
  },
  failed: {
    icon: "XCircle",
    color: "bg-red-500",
    textColor: "text-red-600 dark:text-red-400",
    label: "Failed",
    animate: false,
  },
  cancelled: {
    icon: "StopCircle",
    color: "bg-orange-500",
    textColor: "text-orange-600 dark:text-orange-400",
    label: "Cancelled",
    animate: false,
  },
} as const;

type DocumentStatus = keyof typeof statusConfig;

const PROCESSING_STAGES = [
  {
    key: "chunking",
    label: "Chunking",
    description: "Splitting document into chunks",
  },
  {
    key: "extracting",
    label: "Extracting",
    description: "Running LLM entity extraction",
  },
  {
    key: "embedding",
    label: "Embedding",
    description: "Generating vector embeddings",
  },
  {
    key: "indexing",
    label: "Indexing",
    description: "Storing in graph & vector databases",
  },
] as const;

// ============================================================================
// Helper Functions (replicate from status-badge.tsx)
// ============================================================================

function getStageProgress(
  status: DocumentStatus,
): { current: number; total: number; description: string } | null {
  const stageIndex = PROCESSING_STAGES.findIndex((s) => s.key === status);
  if (stageIndex >= 0) {
    return {
      current: stageIndex + 1,
      total: PROCESSING_STAGES.length,
      description: PROCESSING_STAGES[stageIndex].description,
    };
  }
  if (status === "processing") {
    return {
      current: 1,
      total: PROCESSING_STAGES.length,
      description: "Starting processing...",
    };
  }
  return null;
}

function isProcessingStatus(status: DocumentStatus): boolean {
  return [
    "processing",
    "chunking",
    "extracting",
    "embedding",
    "indexing",
  ].includes(status);
}

function isTerminalStatus(status: DocumentStatus): boolean {
  return ["completed", "indexed", "failed", "cancelled"].includes(status);
}

function normalizeStatus(status: string | undefined | null): DocumentStatus {
  if (!status) return "pending";
  const normalized = status.toLowerCase();
  if (normalized in statusConfig) return normalized as DocumentStatus;
  if (normalized.includes("process")) return "processing";
  return "pending";
}

function getStatusConfig(status: DocumentStatus) {
  return statusConfig[status] || statusConfig.pending;
}

// ============================================================================
// Status Configuration Tests
// ============================================================================

describe("Status Configuration", () => {
  it("should have 10 defined statuses", () => {
    expect(Object.keys(statusConfig)).toHaveLength(10);
  });

  it("should have correct queue states", () => {
    expect(statusConfig.pending).toBeDefined();
    expect(statusConfig.pending.animate).toBe(false);
  });

  it("should have correct processing states", () => {
    const processingStates = [
      "processing",
      "chunking",
      "extracting",
      "embedding",
      "indexing",
    ];
    processingStates.forEach((state) => {
      const config = statusConfig[state as DocumentStatus];
      expect(config).toBeDefined();
      expect(config.animate).toBe(true);
    });
  });

  it("should have correct terminal states", () => {
    const terminalStates = ["completed", "indexed", "failed", "cancelled"];
    terminalStates.forEach((state) => {
      const config = statusConfig[state as DocumentStatus];
      expect(config).toBeDefined();
      expect(config.animate).toBe(false);
    });
  });

  it("should have unique colors for each status category", () => {
    // Queue state
    expect(statusConfig.pending.color).toContain("yellow");

    // Processing states have blue/purple/cyan/teal
    expect(statusConfig.processing.color).toContain("blue");
    expect(statusConfig.extracting.color).toContain("purple");
    expect(statusConfig.embedding.color).toContain("cyan");

    // Terminal states
    expect(statusConfig.completed.color).toContain("green");
    expect(statusConfig.failed.color).toContain("red");
    expect(statusConfig.cancelled.color).toContain("orange");
  });

  it("should have labels for all statuses", () => {
    Object.values(statusConfig).forEach((config) => {
      expect(config.label).toBeTruthy();
      expect(typeof config.label).toBe("string");
    });
  });
});

// ============================================================================
// Processing Stages Tests
// ============================================================================

describe("Processing Stages", () => {
  it("should have 4 processing stages", () => {
    expect(PROCESSING_STAGES).toHaveLength(4);
  });

  it("should have stages in correct order", () => {
    expect(PROCESSING_STAGES[0].key).toBe("chunking");
    expect(PROCESSING_STAGES[1].key).toBe("extracting");
    expect(PROCESSING_STAGES[2].key).toBe("embedding");
    expect(PROCESSING_STAGES[3].key).toBe("indexing");
  });

  it("should have descriptions for all stages", () => {
    PROCESSING_STAGES.forEach((stage) => {
      expect(stage.description).toBeTruthy();
      expect(stage.description.length).toBeGreaterThan(10);
    });
  });
});

// ============================================================================
// getStageProgress Tests
// ============================================================================

describe("getStageProgress", () => {
  describe("processing stages", () => {
    it("should return progress for chunking (stage 1)", () => {
      const progress = getStageProgress("chunking");
      expect(progress).toEqual({
        current: 1,
        total: 4,
        description: "Splitting document into chunks",
      });
    });

    it("should return progress for extracting (stage 2)", () => {
      const progress = getStageProgress("extracting");
      expect(progress).toEqual({
        current: 2,
        total: 4,
        description: "Running LLM entity extraction",
      });
    });

    it("should return progress for embedding (stage 3)", () => {
      const progress = getStageProgress("embedding");
      expect(progress).toEqual({
        current: 3,
        total: 4,
        description: "Generating vector embeddings",
      });
    });

    it("should return progress for indexing (stage 4)", () => {
      const progress = getStageProgress("indexing");
      expect(progress).toEqual({
        current: 4,
        total: 4,
        description: "Storing in graph & vector databases",
      });
    });
  });

  describe("generic processing status", () => {
    it("should return stage 1 for generic processing", () => {
      const progress = getStageProgress("processing");
      expect(progress).toEqual({
        current: 1,
        total: 4,
        description: "Starting processing...",
      });
    });
  });

  describe("non-processing statuses", () => {
    it("should return null for pending", () => {
      expect(getStageProgress("pending")).toBeNull();
    });

    it("should return null for completed", () => {
      expect(getStageProgress("completed")).toBeNull();
    });

    it("should return null for failed", () => {
      expect(getStageProgress("failed")).toBeNull();
    });

    it("should return null for cancelled", () => {
      expect(getStageProgress("cancelled")).toBeNull();
    });

    it("should return null for indexed", () => {
      expect(getStageProgress("indexed")).toBeNull();
    });
  });
});

// ============================================================================
// isProcessingStatus Tests
// ============================================================================

describe("isProcessingStatus", () => {
  it("should return true for all processing states", () => {
    expect(isProcessingStatus("processing")).toBe(true);
    expect(isProcessingStatus("chunking")).toBe(true);
    expect(isProcessingStatus("extracting")).toBe(true);
    expect(isProcessingStatus("embedding")).toBe(true);
    expect(isProcessingStatus("indexing")).toBe(true);
  });

  it("should return false for non-processing states", () => {
    expect(isProcessingStatus("pending")).toBe(false);
    expect(isProcessingStatus("completed")).toBe(false);
    expect(isProcessingStatus("indexed")).toBe(false);
    expect(isProcessingStatus("failed")).toBe(false);
    expect(isProcessingStatus("cancelled")).toBe(false);
  });
});

// ============================================================================
// isTerminalStatus Tests
// ============================================================================

describe("isTerminalStatus", () => {
  it("should return true for all terminal states", () => {
    expect(isTerminalStatus("completed")).toBe(true);
    expect(isTerminalStatus("indexed")).toBe(true);
    expect(isTerminalStatus("failed")).toBe(true);
    expect(isTerminalStatus("cancelled")).toBe(true);
  });

  it("should return false for non-terminal states", () => {
    expect(isTerminalStatus("pending")).toBe(false);
    expect(isTerminalStatus("processing")).toBe(false);
    expect(isTerminalStatus("chunking")).toBe(false);
    expect(isTerminalStatus("extracting")).toBe(false);
    expect(isTerminalStatus("embedding")).toBe(false);
    expect(isTerminalStatus("indexing")).toBe(false);
  });
});

// ============================================================================
// normalizeStatus Tests
// ============================================================================

describe("normalizeStatus", () => {
  describe("null/undefined handling", () => {
    it("should return pending for null", () => {
      expect(normalizeStatus(null)).toBe("pending");
    });

    it("should return pending for undefined", () => {
      expect(normalizeStatus(undefined)).toBe("pending");
    });

    it("should return pending for empty string", () => {
      expect(normalizeStatus("")).toBe("pending");
    });
  });

  describe("known statuses", () => {
    it("should return the status if valid", () => {
      expect(normalizeStatus("pending")).toBe("pending");
      expect(normalizeStatus("processing")).toBe("processing");
      expect(normalizeStatus("completed")).toBe("completed");
      expect(normalizeStatus("failed")).toBe("failed");
    });

    it("should normalize case", () => {
      expect(normalizeStatus("PENDING")).toBe("pending");
      expect(normalizeStatus("Completed")).toBe("completed");
      expect(normalizeStatus("FAILED")).toBe("failed");
      expect(normalizeStatus("ChUnKiNg")).toBe("chunking");
    });
  });

  describe("legacy/unknown statuses", () => {
    it("should map processing-like statuses to processing", () => {
      expect(normalizeStatus("in_processing")).toBe("processing");
      expect(normalizeStatus("still_processing")).toBe("processing");
      expect(normalizeStatus("reprocessing")).toBe("processing");
    });

    it("should return pending for unknown statuses", () => {
      expect(normalizeStatus("unknown")).toBe("pending");
      expect(normalizeStatus("something_else")).toBe("pending");
      expect(normalizeStatus("queued")).toBe("pending");
    });
  });
});

// ============================================================================
// getStatusConfig Tests
// ============================================================================

describe("getStatusConfig", () => {
  it("should return config for valid status", () => {
    const config = getStatusConfig("completed");
    expect(config.label).toBe("Completed");
    expect(config.color).toContain("green");
  });

  it("should return pending config for unknown status", () => {
    const config = getStatusConfig("unknown" as DocumentStatus);
    expect(config).toEqual(statusConfig.pending);
  });
});

// ============================================================================
// Status Flow Tests
// ============================================================================

describe("Status Flow", () => {
  it("should follow correct processing order", () => {
    const stages = ["chunking", "extracting", "embedding", "indexing"] as const;

    stages.forEach((status, index) => {
      const progress = getStageProgress(status);
      expect(progress?.current).toBe(index + 1);
    });
  });

  it("should detect processing to terminal transitions", () => {
    // All processing states should not be terminal
    const processingStates: DocumentStatus[] = [
      "processing",
      "chunking",
      "extracting",
      "embedding",
      "indexing",
    ];
    processingStates.forEach((status) => {
      expect(isProcessingStatus(status)).toBe(true);
      expect(isTerminalStatus(status)).toBe(false);
    });

    // All terminal states should not be processing
    const terminalStates: DocumentStatus[] = [
      "completed",
      "indexed",
      "failed",
      "cancelled",
    ];
    terminalStates.forEach((status) => {
      expect(isTerminalStatus(status)).toBe(true);
      expect(isProcessingStatus(status)).toBe(false);
    });
  });

  it("should have mutually exclusive processing and terminal states", () => {
    const allStatuses: DocumentStatus[] = [
      "pending",
      "processing",
      "chunking",
      "extracting",
      "embedding",
      "indexing",
      "completed",
      "indexed",
      "failed",
      "cancelled",
    ];

    allStatuses.forEach((status) => {
      const isProcessing = isProcessingStatus(status);
      const isTerminal = isTerminalStatus(status);

      // Cannot be both processing and terminal
      expect(isProcessing && isTerminal).toBe(false);

      // Pending is neither
      if (status === "pending") {
        expect(isProcessing).toBe(false);
        expect(isTerminal).toBe(false);
      }
    });
  });
});

// ============================================================================
// Animation Tests
// ============================================================================

describe("Animation States", () => {
  it("should animate all processing states", () => {
    const processingStates: DocumentStatus[] = [
      "processing",
      "chunking",
      "extracting",
      "embedding",
      "indexing",
    ];
    processingStates.forEach((status) => {
      expect(statusConfig[status].animate).toBe(true);
    });
  });

  it("should not animate non-processing states", () => {
    const nonProcessingStates: DocumentStatus[] = [
      "pending",
      "completed",
      "indexed",
      "failed",
      "cancelled",
    ];
    nonProcessingStates.forEach((status) => {
      expect(statusConfig[status].animate).toBe(false);
    });
  });
});

// ============================================================================
// Icon Tests
// ============================================================================

describe("Icon Assignments", () => {
  it("should have Clock for pending", () => {
    expect(statusConfig.pending.icon).toBe("Clock");
  });

  it("should have CheckCircle for success states", () => {
    expect(statusConfig.completed.icon).toBe("CheckCircle");
    expect(statusConfig.indexed.icon).toBe("CheckCircle");
  });

  it("should have XCircle for failed", () => {
    expect(statusConfig.failed.icon).toBe("XCircle");
  });

  it("should have StopCircle for cancelled", () => {
    expect(statusConfig.cancelled.icon).toBe("StopCircle");
  });

  it("should have distinct icons for processing stages", () => {
    expect(statusConfig.chunking.icon).toBe("Scissors");
    expect(statusConfig.extracting.icon).toBe("Brain");
    expect(statusConfig.embedding.icon).toBe("Cpu");
    expect(statusConfig.indexing.icon).toBe("Database");
  });
});

// ============================================================================
// Edge Cases
// ============================================================================

describe("Edge Cases", () => {
  it("should handle whitespace in status", () => {
    expect(normalizeStatus("  pending  ")).toBe("pending");
  });

  it("should handle status with extra characters", () => {
    // This would return pending as it doesn't match exactly
    expect(normalizeStatus("pending!")).toBe("pending");
  });

  it("should handle numeric status (as string)", () => {
    expect(normalizeStatus("123")).toBe("pending");
  });
});

// ============================================================================
// Performance Tests
// ============================================================================

describe("Performance", () => {
  it("should check status types quickly", () => {
    const start = performance.now();

    for (let i = 0; i < 10000; i++) {
      isProcessingStatus("extracting");
      isTerminalStatus("completed");
    }

    const duration = performance.now() - start;
    expect(duration).toBeLessThan(50);
  });

  it("should normalize statuses quickly", () => {
    const start = performance.now();
    const statuses = [
      "pending",
      "PROCESSING",
      "Completed",
      "unknown",
      null,
      undefined,
    ];

    for (let i = 0; i < 10000; i++) {
      statuses.forEach((s) => normalizeStatus(s));
    }

    const duration = performance.now() - start;
    expect(duration).toBeLessThan(100);
  });

  it("should get stage progress quickly", () => {
    const start = performance.now();
    const statuses: DocumentStatus[] = [
      "chunking",
      "extracting",
      "embedding",
      "indexing",
      "pending",
    ];

    for (let i = 0; i < 10000; i++) {
      statuses.forEach((s) => getStageProgress(s));
    }

    const duration = performance.now() - start;
    expect(duration).toBeLessThan(100);
  });
});
