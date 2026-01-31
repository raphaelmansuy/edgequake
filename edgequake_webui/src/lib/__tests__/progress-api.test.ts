/**
 * @module progress-api.test
 * @description Tests for progress tracking API types and serialization
 *
 * @implements OODA-42: Progress API tests
 *
 * Tests cover:
 * - API response types
 * - Serialization/deserialization
 * - Type guards
 * - Default values
 */

import { describe, expect, it } from "vitest";

// ============================================================================
// Types (matching backend API types)
// ============================================================================

type PipelinePhase =
  | "Upload"
  | "PdfConversion"
  | "Chunking"
  | "Embedding"
  | "Extraction"
  | "GraphStorage";

interface PhaseProgress {
  phase: PipelinePhase;
  status: "pending" | "active" | "complete" | "failed";
  current: number;
  total: number;
  percentage: number;
  eta_seconds?: number;
  message: string;
  error?: PhaseError;
}

interface PhaseError {
  code: string;
  message: string;
  recoverable: boolean;
}

interface UploadProgress {
  track_id: string;
  phases: PhaseProgress[];
  overall_percentage: number;
  started_at: string;
  updated_at: string;
  completed_at?: string;
}

// ============================================================================
// Type Guards
// ============================================================================

function isValidPhase(phase: string): phase is PipelinePhase {
  const validPhases: PipelinePhase[] = [
    "Upload",
    "PdfConversion",
    "Chunking",
    "Embedding",
    "Extraction",
    "GraphStorage",
  ];
  return validPhases.includes(phase as PipelinePhase);
}

function isValidStatus(status: string): status is PhaseProgress["status"] {
  return ["pending", "active", "complete", "failed"].includes(status);
}

function isPhaseProgress(obj: unknown): obj is PhaseProgress {
  if (typeof obj !== "object" || obj === null) return false;

  const p = obj as Record<string, unknown>;

  return (
    typeof p.phase === "string" &&
    isValidPhase(p.phase) &&
    typeof p.status === "string" &&
    isValidStatus(p.status) &&
    typeof p.current === "number" &&
    typeof p.total === "number" &&
    typeof p.percentage === "number" &&
    typeof p.message === "string"
  );
}

function isUploadProgress(obj: unknown): obj is UploadProgress {
  if (typeof obj !== "object" || obj === null) return false;

  const u = obj as Record<string, unknown>;

  return (
    typeof u.track_id === "string" &&
    Array.isArray(u.phases) &&
    typeof u.overall_percentage === "number" &&
    typeof u.started_at === "string" &&
    typeof u.updated_at === "string"
  );
}

// ============================================================================
// Serialization
// ============================================================================

function serializeProgress(progress: UploadProgress): string {
  return JSON.stringify(progress);
}

function deserializeProgress(json: string): UploadProgress | null {
  try {
    const parsed = JSON.parse(json);
    if (isUploadProgress(parsed)) {
      return parsed;
    }
    return null;
  } catch {
    return null;
  }
}

// ============================================================================
// Default Values
// ============================================================================

function createDefaultPhase(phase: PipelinePhase): PhaseProgress {
  return {
    phase,
    status: "pending",
    current: 0,
    total: 0,
    percentage: 0,
    message: "",
  };
}

function createDefaultProgress(trackId: string): UploadProgress {
  const now = new Date().toISOString();
  return {
    track_id: trackId,
    phases: [
      createDefaultPhase("Upload"),
      createDefaultPhase("PdfConversion"),
      createDefaultPhase("Chunking"),
      createDefaultPhase("Embedding"),
      createDefaultPhase("Extraction"),
      createDefaultPhase("GraphStorage"),
    ],
    overall_percentage: 0,
    started_at: now,
    updated_at: now,
  };
}

// ============================================================================
// Tests
// ============================================================================

describe("Pipeline Phase Validation", () => {
  describe("isValidPhase", () => {
    it("accepts all valid phases", () => {
      const phases: PipelinePhase[] = [
        "Upload",
        "PdfConversion",
        "Chunking",
        "Embedding",
        "Extraction",
        "GraphStorage",
      ];

      for (const phase of phases) {
        expect(isValidPhase(phase)).toBe(true);
      }
    });

    it("rejects invalid phases", () => {
      expect(isValidPhase("InvalidPhase")).toBe(false);
      expect(isValidPhase("")).toBe(false);
      expect(isValidPhase("upload")).toBe(false); // Case-sensitive
    });
  });

  describe("isValidStatus", () => {
    it("accepts all valid statuses", () => {
      expect(isValidStatus("pending")).toBe(true);
      expect(isValidStatus("active")).toBe(true);
      expect(isValidStatus("complete")).toBe(true);
      expect(isValidStatus("failed")).toBe(true);
    });

    it("rejects invalid statuses", () => {
      expect(isValidStatus("PENDING")).toBe(false);
      expect(isValidStatus("running")).toBe(false);
      expect(isValidStatus("")).toBe(false);
    });
  });
});

describe("Type Guards", () => {
  describe("isPhaseProgress", () => {
    it("accepts valid PhaseProgress", () => {
      const phase: PhaseProgress = {
        phase: "Upload",
        status: "active",
        current: 50,
        total: 100,
        percentage: 50,
        message: "Uploading...",
      };

      expect(isPhaseProgress(phase)).toBe(true);
    });

    it("accepts PhaseProgress with optional fields", () => {
      const phase: PhaseProgress = {
        phase: "Extraction",
        status: "failed",
        current: 5,
        total: 10,
        percentage: 50,
        message: "Entity extraction failed",
        eta_seconds: 120,
        error: {
          code: "llm_error",
          message: "API timeout",
          recoverable: true,
        },
      };

      expect(isPhaseProgress(phase)).toBe(true);
    });

    it("rejects invalid phase name", () => {
      const invalid = {
        phase: "BadPhase",
        status: "active",
        current: 0,
        total: 0,
        percentage: 0,
        message: "",
      };

      expect(isPhaseProgress(invalid)).toBe(false);
    });

    it("rejects missing fields", () => {
      expect(isPhaseProgress({})).toBe(false);
      expect(isPhaseProgress({ phase: "Upload" })).toBe(false);
      expect(isPhaseProgress(null)).toBe(false);
      expect(isPhaseProgress(undefined)).toBe(false);
    });

    it("rejects wrong field types", () => {
      const wrongTypes = {
        phase: "Upload",
        status: "active",
        current: "50", // Should be number
        total: 100,
        percentage: 50,
        message: "Test",
      };

      expect(isPhaseProgress(wrongTypes)).toBe(false);
    });
  });

  describe("isUploadProgress", () => {
    it("accepts valid UploadProgress", () => {
      const progress: UploadProgress = {
        track_id: "track-123",
        phases: [],
        overall_percentage: 0,
        started_at: "2025-01-27T00:00:00Z",
        updated_at: "2025-01-27T00:01:00Z",
      };

      expect(isUploadProgress(progress)).toBe(true);
    });

    it("accepts UploadProgress with completed_at", () => {
      const progress: UploadProgress = {
        track_id: "track-456",
        phases: [],
        overall_percentage: 100,
        started_at: "2025-01-27T00:00:00Z",
        updated_at: "2025-01-27T00:05:00Z",
        completed_at: "2025-01-27T00:05:00Z",
      };

      expect(isUploadProgress(progress)).toBe(true);
    });

    it("rejects missing required fields", () => {
      expect(isUploadProgress({})).toBe(false);
      expect(isUploadProgress({ track_id: "x" })).toBe(false);
    });
  });
});

describe("Serialization", () => {
  describe("serializeProgress", () => {
    it("serializes UploadProgress to JSON", () => {
      const progress: UploadProgress = {
        track_id: "track-ser",
        phases: [
          {
            phase: "Upload",
            status: "complete",
            current: 100,
            total: 100,
            percentage: 100,
            message: "Complete",
          },
        ],
        overall_percentage: 16.67,
        started_at: "2025-01-27T00:00:00Z",
        updated_at: "2025-01-27T00:01:00Z",
      };

      const json = serializeProgress(progress);
      expect(typeof json).toBe("string");
      expect(json).toContain("track-ser");
    });
  });

  describe("deserializeProgress", () => {
    it("deserializes valid JSON to UploadProgress", () => {
      const json = JSON.stringify({
        track_id: "track-deser",
        phases: [],
        overall_percentage: 50,
        started_at: "2025-01-27T00:00:00Z",
        updated_at: "2025-01-27T00:02:00Z",
      });

      const result = deserializeProgress(json);
      expect(result).not.toBeNull();
      expect(result?.track_id).toBe("track-deser");
    });

    it("returns null for invalid JSON", () => {
      expect(deserializeProgress("not json")).toBeNull();
      expect(deserializeProgress("{")).toBeNull();
    });

    it("returns null for invalid structure", () => {
      expect(deserializeProgress(JSON.stringify({}))).toBeNull();
      expect(deserializeProgress(JSON.stringify({ foo: "bar" }))).toBeNull();
    });

    it("roundtrips correctly", () => {
      const original: UploadProgress = {
        track_id: "roundtrip-test",
        phases: [
          {
            phase: "PdfConversion",
            status: "active",
            current: 5,
            total: 10,
            percentage: 50,
            message: "Converting page 5 of 10",
          },
        ],
        overall_percentage: 25,
        started_at: "2025-01-27T10:00:00Z",
        updated_at: "2025-01-27T10:05:00Z",
      };

      const json = serializeProgress(original);
      const restored = deserializeProgress(json);

      expect(restored).toEqual(original);
    });
  });
});

describe("Default Values", () => {
  describe("createDefaultPhase", () => {
    it("creates pending phase with zeros", () => {
      const phase = createDefaultPhase("Upload");

      expect(phase.phase).toBe("Upload");
      expect(phase.status).toBe("pending");
      expect(phase.current).toBe(0);
      expect(phase.total).toBe(0);
      expect(phase.percentage).toBe(0);
      expect(phase.message).toBe("");
      expect(phase.error).toBeUndefined();
    });

    it("creates default for each phase type", () => {
      const phases: PipelinePhase[] = [
        "Upload",
        "PdfConversion",
        "Chunking",
        "Embedding",
        "Extraction",
        "GraphStorage",
      ];

      for (const phaseName of phases) {
        const phase = createDefaultPhase(phaseName);
        expect(phase.phase).toBe(phaseName);
      }
    });
  });

  describe("createDefaultProgress", () => {
    it("creates progress with all 6 phases", () => {
      const progress = createDefaultProgress("track-default");

      expect(progress.track_id).toBe("track-default");
      expect(progress.phases).toHaveLength(6);
      expect(progress.overall_percentage).toBe(0);
    });

    it("sets timestamps", () => {
      const before = new Date().toISOString();
      const progress = createDefaultProgress("track-time");
      const after = new Date().toISOString();

      expect(progress.started_at >= before).toBe(true);
      expect(progress.started_at <= after).toBe(true);
      expect(progress.started_at).toBe(progress.updated_at);
    });

    it("has phases in correct order", () => {
      const progress = createDefaultProgress("track-order");
      const phaseNames = progress.phases.map((p) => p.phase);

      expect(phaseNames).toEqual([
        "Upload",
        "PdfConversion",
        "Chunking",
        "Embedding",
        "Extraction",
        "GraphStorage",
      ]);
    });
  });
});

describe("API Response Compatibility", () => {
  describe("mock backend response parsing", () => {
    it("parses realistic backend response", () => {
      const backendResponse = {
        track_id: "pdf-upload-1706400000000",
        phases: [
          {
            phase: "Upload",
            status: "complete",
            current: 1,
            total: 1,
            percentage: 100,
            message: "File uploaded",
          },
          {
            phase: "PdfConversion",
            status: "active",
            current: 3,
            total: 5,
            percentage: 60,
            message: "Converting page 3 of 5",
          },
          {
            phase: "Chunking",
            status: "pending",
            current: 0,
            total: 0,
            percentage: 0,
            message: "",
          },
          {
            phase: "Embedding",
            status: "pending",
            current: 0,
            total: 0,
            percentage: 0,
            message: "",
          },
          {
            phase: "Extraction",
            status: "pending",
            current: 0,
            total: 0,
            percentage: 0,
            message: "",
          },
          {
            phase: "GraphStorage",
            status: "pending",
            current: 0,
            total: 0,
            percentage: 0,
            message: "",
          },
        ],
        overall_percentage: 26.67,
        started_at: "2025-01-27T12:00:00.000Z",
        updated_at: "2025-01-27T12:00:15.000Z",
      };

      const json = JSON.stringify(backendResponse);
      const parsed = deserializeProgress(json);

      expect(parsed).not.toBeNull();
      expect(parsed?.phases[1].status).toBe("active");
      expect(parsed?.overall_percentage).toBeCloseTo(26.67, 2);
    });

    it("parses completed upload response", () => {
      const completedResponse = {
        track_id: "pdf-complete-123",
        phases: [
          {
            phase: "Upload",
            status: "complete",
            current: 1,
            total: 1,
            percentage: 100,
            message: "Done",
          },
          {
            phase: "PdfConversion",
            status: "complete",
            current: 10,
            total: 10,
            percentage: 100,
            message: "Done",
          },
          {
            phase: "Chunking",
            status: "complete",
            current: 25,
            total: 25,
            percentage: 100,
            message: "Done",
          },
          {
            phase: "Embedding",
            status: "complete",
            current: 25,
            total: 25,
            percentage: 100,
            message: "Done",
          },
          {
            phase: "Extraction",
            status: "complete",
            current: 45,
            total: 45,
            percentage: 100,
            message: "Done",
          },
          {
            phase: "GraphStorage",
            status: "complete",
            current: 68,
            total: 68,
            percentage: 100,
            message: "Done",
          },
        ],
        overall_percentage: 100,
        started_at: "2025-01-27T12:00:00.000Z",
        updated_at: "2025-01-27T12:02:30.000Z",
        completed_at: "2025-01-27T12:02:30.000Z",
      };

      const json = JSON.stringify(completedResponse);
      const parsed = deserializeProgress(json);

      expect(parsed?.completed_at).toBeDefined();
      expect(parsed?.overall_percentage).toBe(100);
    });

    it("parses failed upload response", () => {
      const failedResponse = {
        track_id: "pdf-failed-456",
        phases: [
          {
            phase: "Upload",
            status: "complete",
            current: 1,
            total: 1,
            percentage: 100,
            message: "Done",
          },
          {
            phase: "PdfConversion",
            status: "failed",
            current: 3,
            total: 10,
            percentage: 30,
            message: "Failed at page 3",
            error: {
              code: "corrupt_page",
              message: "Cannot decode page 3: invalid object stream",
              recoverable: false,
            },
          },
          {
            phase: "Chunking",
            status: "pending",
            current: 0,
            total: 0,
            percentage: 0,
            message: "",
          },
          {
            phase: "Embedding",
            status: "pending",
            current: 0,
            total: 0,
            percentage: 0,
            message: "",
          },
          {
            phase: "Extraction",
            status: "pending",
            current: 0,
            total: 0,
            percentage: 0,
            message: "",
          },
          {
            phase: "GraphStorage",
            status: "pending",
            current: 0,
            total: 0,
            percentage: 0,
            message: "",
          },
        ],
        overall_percentage: 21.67,
        started_at: "2025-01-27T12:00:00.000Z",
        updated_at: "2025-01-27T12:00:45.000Z",
      };

      const json = JSON.stringify(failedResponse);
      const parsed = deserializeProgress(json);

      expect(parsed?.phases[1].status).toBe("failed");
      expect(parsed?.phases[1].error?.code).toBe("corrupt_page");
    });
  });
});
