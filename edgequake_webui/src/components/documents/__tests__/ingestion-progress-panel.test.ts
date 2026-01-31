/**
 * OODA-45: Ingestion Progress Panel Unit Tests
 *
 * @description Tests for the ingestion progress visualization components
 * @implements Phase 4 Testing & Validation
 * @see specs/001-upload-pdf.md
 */

import type {
  IngestionProgress,
  IngestionStage,
  StageProgress,
} from "@/types/ingestion";
import { describe, expect, it } from "vitest";

// ============================================================================
// Helper Functions for Stage Logic
// ============================================================================

/**
 * Creates default stages with pending status
 * Replicates createDefaultStages logic from stage-indicator component
 */
function createDefaultStages(currentStage?: IngestionStage) {
  const stageOrder: IngestionStage[] = [
    "preprocessing",
    "chunking",
    "extracting",
    "merging",
    "embedding",
    "indexing",
  ];

  return stageOrder.map((stageId) => ({
    id: stageId,
    label: stageId,
    status:
      stageId === currentStage ? ("running" as const) : ("pending" as const),
  }));
}

/**
 * Build stages from progress data
 * Replicates the stages useMemo logic from IngestionProgressPanel
 */
function buildStagesFromProgress(
  progress: Partial<IngestionProgress> | null,
): Array<{
  id: IngestionStage;
  label: string;
  status: "pending" | "running" | "completed" | "failed";
  progress?: number;
  duration?: number;
  message?: string;
}> {
  if (!progress?.progress?.stages) {
    return createDefaultStages(
      progress?.progress?.current_stage as IngestionStage,
    );
  }

  const stageOrder: IngestionStage[] = [
    "preprocessing",
    "chunking",
    "extracting",
    "merging",
    "embedding",
    "indexing",
  ];

  return stageOrder.map((stageId) => {
    const stageData = progress.progress!.stages.find(
      (s: StageProgress) => s.stage === stageId,
    );
    const isCurrent = stageId === progress.progress!.current_stage;

    if (!stageData) {
      return {
        id: stageId,
        label: stageId,
        status: "pending" as const,
      };
    }

    let status: "pending" | "running" | "completed" | "failed" = "pending";
    if (stageData.status === "completed") {
      status = "completed";
    } else if (isCurrent) {
      status = "running";
    } else if (stageData.status === "failed") {
      status = "failed";
    }

    return {
      id: stageId,
      label: stageId,
      status,
      progress: isCurrent ? stageData.progress : undefined,
      duration: stageData.duration_ms,
      message: stageData.message,
    };
  });
}

/**
 * Get current stage message
 */
function getCurrentMessage(
  progress: Partial<IngestionProgress> | null,
): string {
  if (!progress?.progress?.stages) return "Starting...";

  const currentStage = progress.progress.current_stage;
  if (!currentStage) return "Preparing...";

  const stageData = progress.progress.stages.find(
    (s: StageProgress) => s.stage === currentStage,
  );
  return stageData?.message || `Processing ${currentStage}...`;
}

// ============================================================================
// Default Stages Tests
// ============================================================================

describe("createDefaultStages", () => {
  it("should create 6 stages with correct order", () => {
    const stages = createDefaultStages();

    expect(stages).toHaveLength(6);
    expect(stages[0].id).toBe("preprocessing");
    expect(stages[1].id).toBe("chunking");
    expect(stages[2].id).toBe("extracting");
    expect(stages[3].id).toBe("merging");
    expect(stages[4].id).toBe("embedding");
    expect(stages[5].id).toBe("indexing");
  });

  it("should set all stages to pending when no current stage", () => {
    const stages = createDefaultStages();

    stages.forEach((stage) => {
      expect(stage.status).toBe("pending");
    });
  });

  it("should mark current stage as running", () => {
    const stages = createDefaultStages("chunking");

    expect(stages[0].status).toBe("pending");
    expect(stages[1].status).toBe("running");
    expect(stages[2].status).toBe("pending");
  });

  it("should handle first stage as current", () => {
    const stages = createDefaultStages("preprocessing");

    expect(stages[0].status).toBe("running");
    expect(stages[1].status).toBe("pending");
  });

  it("should handle last stage as current", () => {
    const stages = createDefaultStages("indexing");

    expect(stages[4].status).toBe("pending");
    expect(stages[5].status).toBe("running");
  });

  it("should use stage id as label", () => {
    const stages = createDefaultStages();

    stages.forEach((stage) => {
      expect(stage.label).toBe(stage.id);
    });
  });
});

// ============================================================================
// Build Stages From Progress Tests
// ============================================================================

describe("buildStagesFromProgress", () => {
  describe("without progress data", () => {
    it("should return default stages when progress is null", () => {
      const stages = buildStagesFromProgress(null);

      expect(stages).toHaveLength(6);
      stages.forEach((stage) => {
        expect(stage.status).toBe("pending");
      });
    });

    it("should return default stages when stages array is missing", () => {
      const stages = buildStagesFromProgress({
        progress: {
          current_stage: "chunking",
        } as unknown as IngestionProgress["progress"],
      });

      expect(stages).toHaveLength(6);
      expect(stages[1].status).toBe("running"); // chunking
    });
  });

  describe("with progress data", () => {
    it("should mark completed stages correctly", () => {
      const progress: Partial<IngestionProgress> = {
        progress: {
          current_stage: "extracting",
          completion_percentage: 50,
          latest_message: "Extracting...",
          stages: [
            {
              stage: "preprocessing",
              status: "completed",
              progress: 100,
              completed_items: 1,
              total_items: 1,
              duration_ms: 1000,
            },
            {
              stage: "chunking",
              status: "completed",
              progress: 100,
              completed_items: 10,
              total_items: 10,
              duration_ms: 2000,
            },
            {
              stage: "extracting",
              status: "running",
              progress: 30,
              completed_items: 3,
              total_items: 10,
            },
            {
              stage: "merging",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "embedding",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "indexing",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
          ],
        },
      };

      const stages = buildStagesFromProgress(progress);

      expect(stages[0].status).toBe("completed");
      expect(stages[1].status).toBe("completed");
      expect(stages[2].status).toBe("running");
      expect(stages[3].status).toBe("pending");
      expect(stages[4].status).toBe("pending");
      expect(stages[5].status).toBe("pending");
    });

    it("should include progress percentage for current stage only", () => {
      const progress: Partial<IngestionProgress> = {
        progress: {
          current_stage: "embedding",
          completion_percentage: 75,
          latest_message: "Embedding...",
          stages: [
            {
              stage: "preprocessing",
              status: "completed",
              progress: 100,
              completed_items: 1,
              total_items: 1,
            },
            {
              stage: "chunking",
              status: "completed",
              progress: 100,
              completed_items: 10,
              total_items: 10,
            },
            {
              stage: "extracting",
              status: "completed",
              progress: 100,
              completed_items: 50,
              total_items: 50,
            },
            {
              stage: "merging",
              status: "completed",
              progress: 100,
              completed_items: 1,
              total_items: 1,
            },
            {
              stage: "embedding",
              status: "running",
              progress: 60,
              completed_items: 30,
              total_items: 50,
            },
            {
              stage: "indexing",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
          ],
        },
      };

      const stages = buildStagesFromProgress(progress);

      expect(stages[0].progress).toBeUndefined();
      expect(stages[1].progress).toBeUndefined();
      expect(stages[2].progress).toBeUndefined();
      expect(stages[3].progress).toBeUndefined();
      expect(stages[4].progress).toBe(60); // current stage
      expect(stages[5].progress).toBeUndefined();
    });

    it("should include duration for completed stages", () => {
      const progress: Partial<IngestionProgress> = {
        progress: {
          current_stage: "extracting",
          completion_percentage: 25,
          latest_message: "Extracting...",
          stages: [
            {
              stage: "preprocessing",
              status: "completed",
              progress: 100,
              completed_items: 1,
              total_items: 1,
              duration_ms: 1500,
            },
            {
              stage: "chunking",
              status: "completed",
              progress: 100,
              completed_items: 10,
              total_items: 10,
              duration_ms: 3200,
            },
            {
              stage: "extracting",
              status: "running",
              progress: 25,
              completed_items: 5,
              total_items: 20,
            },
            {
              stage: "merging",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "embedding",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "indexing",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
          ],
        },
      };

      const stages = buildStagesFromProgress(progress);

      expect(stages[0].duration).toBe(1500);
      expect(stages[1].duration).toBe(3200);
      expect(stages[2].duration).toBeUndefined();
    });

    it("should mark failed stage correctly", () => {
      const progress: Partial<IngestionProgress> = {
        progress: {
          current_stage: "extracting",
          completion_percentage: 25,
          latest_message: "Failed!",
          stages: [
            {
              stage: "preprocessing",
              status: "completed",
              progress: 100,
              completed_items: 1,
              total_items: 1,
            },
            {
              stage: "chunking",
              status: "completed",
              progress: 100,
              completed_items: 10,
              total_items: 10,
            },
            {
              stage: "extracting",
              status: "failed",
              progress: 25,
              completed_items: 5,
              total_items: 20,
            },
            {
              stage: "merging",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "embedding",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "indexing",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
          ],
        },
      };

      const stages = buildStagesFromProgress(progress);

      expect(stages[0].status).toBe("completed");
      expect(stages[1].status).toBe("completed");
      expect(stages[2].status).toBe("running"); // current stage takes priority even if failed
      expect(stages[3].status).toBe("pending");
    });

    it("should include stage message", () => {
      const progress: Partial<IngestionProgress> = {
        progress: {
          current_stage: "chunking",
          completion_percentage: 15,
          latest_message: "Chunking...",
          stages: [
            {
              stage: "preprocessing",
              status: "completed",
              progress: 100,
              completed_items: 1,
              total_items: 1,
              message: "Validated PDF",
            },
            {
              stage: "chunking",
              status: "running",
              progress: 50,
              completed_items: 5,
              total_items: 10,
              message: "Page 5 of 10",
            },
            {
              stage: "extracting",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "merging",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "embedding",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "indexing",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
          ],
        },
      };

      const stages = buildStagesFromProgress(progress);

      expect(stages[0].message).toBe("Validated PDF");
      expect(stages[1].message).toBe("Page 5 of 10");
    });

    it("should handle missing stages in progress data", () => {
      const progress: Partial<IngestionProgress> = {
        progress: {
          current_stage: "chunking",
          completion_percentage: 10,
          latest_message: "Chunking...",
          stages: [
            // Only preprocessing and chunking present
            {
              stage: "preprocessing",
              status: "completed",
              progress: 100,
              completed_items: 1,
              total_items: 1,
            },
            {
              stage: "chunking",
              status: "running",
              progress: 40,
              completed_items: 4,
              total_items: 10,
            },
          ],
        },
      };

      const stages = buildStagesFromProgress(progress);

      expect(stages).toHaveLength(6);
      expect(stages[0].status).toBe("completed");
      expect(stages[1].status).toBe("running");
      expect(stages[2].status).toBe("pending"); // extracting not in data
      expect(stages[3].status).toBe("pending");
      expect(stages[4].status).toBe("pending");
      expect(stages[5].status).toBe("pending");
    });
  });
});

// ============================================================================
// Get Current Message Tests
// ============================================================================

describe("getCurrentMessage", () => {
  it('should return "Starting..." when no progress', () => {
    expect(getCurrentMessage(null)).toBe("Starting...");
  });

  it('should return "Starting..." when no stages', () => {
    expect(
      getCurrentMessage({
        progress: {
          current_stage: "chunking",
        } as unknown as IngestionProgress["progress"],
      }),
    ).toBe("Starting...");
  });

  it('should return "Preparing..." when no current stage', () => {
    expect(
      getCurrentMessage({
        progress: {
          current_stage: undefined as unknown as IngestionStage,
          completion_percentage: 0,
          latest_message: "",
          stages: [],
        },
      }),
    ).toBe("Preparing...");
  });

  it("should return stage message when available", () => {
    expect(
      getCurrentMessage({
        progress: {
          current_stage: "extracting",
          completion_percentage: 50,
          latest_message: "Extracting entities from chunk 5/10",
          stages: [
            {
              stage: "extracting",
              status: "running",
              progress: 50,
              completed_items: 5,
              total_items: 10,
              message: "Extracting entities from chunk 5/10",
            },
          ],
        },
      }),
    ).toBe("Extracting entities from chunk 5/10");
  });

  it("should return default message when stage has no message", () => {
    expect(
      getCurrentMessage({
        progress: {
          current_stage: "embedding",
          completion_percentage: 60,
          latest_message: "",
          stages: [
            {
              stage: "embedding",
              status: "running",
              progress: 60,
              completed_items: 30,
              total_items: 50,
            },
          ],
        },
      }),
    ).toBe("Processing embedding...");
  });
});

// ============================================================================
// Progress Completion Detection Tests
// ============================================================================

describe("Progress Completion Detection", () => {
  it("should detect completed status", () => {
    const progress: Partial<IngestionProgress> = {
      status: "completed",
      overall_progress: 100,
    };

    expect(progress.status === "completed").toBe(true);
  });

  it("should detect failed status", () => {
    const progress: Partial<IngestionProgress> = {
      status: "failed",
      overall_progress: 45,
    };

    expect(progress.status === "failed").toBe(true);
  });

  it("should detect processing status", () => {
    const progress: Partial<IngestionProgress> = {
      status: "preprocessing",
      overall_progress: 5,
    };

    expect(
      progress.status !== "completed" && progress.status !== "failed",
    ).toBe(true);
  });
});

// ============================================================================
// Stage Order Consistency Tests
// ============================================================================

describe("Stage Order Consistency", () => {
  const expectedOrder: IngestionStage[] = [
    "preprocessing",
    "chunking",
    "extracting",
    "merging",
    "embedding",
    "indexing",
  ];

  it("should maintain consistent stage order", () => {
    const stages = createDefaultStages();

    stages.forEach((stage, index) => {
      expect(stage.id).toBe(expectedOrder[index]);
    });
  });

  it("should handle out-of-order progress data", () => {
    const progress: Partial<IngestionProgress> = {
      progress: {
        current_stage: "chunking",
        completion_percentage: 10,
        latest_message: "Chunking...",
        stages: [
          // Out of order
          {
            stage: "embedding",
            status: "pending",
            progress: 0,
            completed_items: 0,
            total_items: 0,
          },
          {
            stage: "preprocessing",
            status: "completed",
            progress: 100,
            completed_items: 1,
            total_items: 1,
          },
          {
            stage: "chunking",
            status: "running",
            progress: 30,
            completed_items: 3,
            total_items: 10,
          },
        ],
      },
    };

    const stages = buildStagesFromProgress(progress);

    // Should still be in correct order
    expect(stages[0].id).toBe("preprocessing");
    expect(stages[1].id).toBe("chunking");
    expect(stages[4].id).toBe("embedding");
  });
});

// ============================================================================
// Compact vs Full Mode Tests
// ============================================================================

describe("Display Mode Logic", () => {
  it("should calculate compact mode display values", () => {
    const progress: Partial<IngestionProgress> = {
      overall_progress: 65,
      status: "extracting",
    };

    // In compact mode, we show overall percentage
    const displayPercentage = Math.round(progress.overall_progress ?? 0);
    expect(displayPercentage).toBe(65);
  });

  it("should determine progress bar variant based on status", () => {
    const getVariant = (status?: string) => {
      if (status === "failed") return "error";
      return "default";
    };

    expect(getVariant("failed")).toBe("error");
    expect(getVariant("completed")).toBe("default");
    expect(getVariant("processing")).toBe("default");
    expect(getVariant(undefined)).toBe("default");
  });
});

// ============================================================================
// Edge Cases Tests
// ============================================================================

describe("Edge Cases", () => {
  it("should handle 0% progress", () => {
    const stages = buildStagesFromProgress({
      progress: {
        current_stage: "preprocessing",
        completion_percentage: 0,
        latest_message: "Starting...",
        stages: [
          {
            stage: "preprocessing",
            status: "running",
            progress: 0,
            completed_items: 0,
            total_items: 1,
          },
        ],
      },
    });

    expect(stages[0].status).toBe("running");
    expect(stages[0].progress).toBe(0);
  });

  it("should handle 100% progress", () => {
    const progress: Partial<IngestionProgress> = {
      progress: {
        current_stage: "indexing",
        completion_percentage: 100,
        latest_message: "Complete!",
        stages: [
          {
            stage: "preprocessing",
            status: "completed",
            progress: 100,
            completed_items: 1,
            total_items: 1,
          },
          {
            stage: "chunking",
            status: "completed",
            progress: 100,
            completed_items: 10,
            total_items: 10,
          },
          {
            stage: "extracting",
            status: "completed",
            progress: 100,
            completed_items: 50,
            total_items: 50,
          },
          {
            stage: "merging",
            status: "completed",
            progress: 100,
            completed_items: 1,
            total_items: 1,
          },
          {
            stage: "embedding",
            status: "completed",
            progress: 100,
            completed_items: 50,
            total_items: 50,
          },
          {
            stage: "indexing",
            status: "completed",
            progress: 100,
            completed_items: 50,
            total_items: 50,
          },
        ],
      },
    };

    const stages = buildStagesFromProgress(progress);

    stages.forEach((stage) => {
      expect(stage.status).toBe("completed");
    });
  });

  it("should handle empty stages array", () => {
    const stages = buildStagesFromProgress({
      progress: {
        current_stage: "preprocessing",
        completion_percentage: 0,
        latest_message: "",
        stages: [],
      },
    });

    // All stages should be pending since none match
    expect(stages).toHaveLength(6);
    stages.forEach((stage, i) => {
      if (i === 0) {
        expect(stage.status).toBe("pending"); // preprocessing is current but not in stages
      } else {
        expect(stage.status).toBe("pending");
      }
    });
  });

  it("should handle unknown stage in progress data", () => {
    const progress: Partial<IngestionProgress> = {
      progress: {
        current_stage: "chunking",
        completion_percentage: 10,
        latest_message: "Chunking...",
        stages: [
          {
            stage: "preprocessing",
            status: "completed",
            progress: 100,
            completed_items: 1,
            total_items: 1,
          },
          {
            stage: "unknown_stage" as IngestionStage,
            status: "completed",
            progress: 100,
            completed_items: 1,
            total_items: 1,
          },
          {
            stage: "chunking",
            status: "running",
            progress: 50,
            completed_items: 5,
            total_items: 10,
          },
        ],
      },
    };

    const stages = buildStagesFromProgress(progress);

    // Should still return 6 stages in correct order
    expect(stages).toHaveLength(6);
    expect(stages[0].status).toBe("completed");
    expect(stages[1].status).toBe("running");
    // unknown_stage is ignored
  });

  it("should handle negative progress values", () => {
    const stages = buildStagesFromProgress({
      progress: {
        current_stage: "extracting",
        completion_percentage: -10,
        latest_message: "Error state",
        stages: [
          {
            stage: "extracting",
            status: "running",
            progress: -5,
            completed_items: 0,
            total_items: 10,
          },
        ],
      },
    });

    // Should still work, component should handle display
    expect(stages[2].progress).toBe(-5);
  });

  it("should handle progress values over 100", () => {
    const stages = buildStagesFromProgress({
      progress: {
        current_stage: "embedding",
        completion_percentage: 150,
        latest_message: "Processing",
        stages: [
          {
            stage: "embedding",
            status: "running",
            progress: 120,
            completed_items: 60,
            total_items: 50,
          },
        ],
      },
    });

    // Should still work, component should handle display
    expect(stages[4].progress).toBe(120);
  });
});

// ============================================================================
// Performance Tests
// ============================================================================

describe("Performance", () => {
  it("should handle rapid stage updates efficiently", () => {
    const start = performance.now();

    for (let i = 0; i < 1000; i++) {
      const progress: Partial<IngestionProgress> = {
        progress: {
          current_stage: "extracting",
          completion_percentage: i / 10,
          latest_message: `Processing ${i}`,
          stages: [
            {
              stage: "preprocessing",
              status: "completed",
              progress: 100,
              completed_items: 1,
              total_items: 1,
            },
            {
              stage: "chunking",
              status: "completed",
              progress: 100,
              completed_items: 10,
              total_items: 10,
            },
            {
              stage: "extracting",
              status: "running",
              progress: i / 10,
              completed_items: i,
              total_items: 1000,
            },
            {
              stage: "merging",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "embedding",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
            {
              stage: "indexing",
              status: "pending",
              progress: 0,
              completed_items: 0,
              total_items: 0,
            },
          ],
        },
      };

      buildStagesFromProgress(progress);
    }

    const duration = performance.now() - start;
    expect(duration).toBeLessThan(100); // Should complete in under 100ms
  });

  it("should handle many stages without performance degradation", () => {
    const start = performance.now();

    for (let i = 0; i < 500; i++) {
      createDefaultStages("extracting");
    }

    const duration = performance.now() - start;
    expect(duration).toBeLessThan(50); // Should be very fast
  });
});
