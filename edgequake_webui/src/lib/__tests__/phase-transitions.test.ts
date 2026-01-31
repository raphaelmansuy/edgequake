/**
 * @module phase-transitions.test
 * @description Tests for pipeline phase transition logic
 *
 * @implements OODA-43: Phase transition tests
 *
 * Tests cover:
 * - Valid phase transitions
 * - Invalid transition prevention
 * - Phase completion requirements
 * - Phase ordering rules
 */

import { describe, expect, it } from "vitest";

// ============================================================================
// Types
// ============================================================================

type PipelinePhase =
  | "Upload"
  | "PdfConversion"
  | "Chunking"
  | "Embedding"
  | "Extraction"
  | "GraphStorage";

type PhaseStatus = "pending" | "active" | "complete" | "failed";

interface PhaseState {
  phase: PipelinePhase;
  status: PhaseStatus;
}

// ============================================================================
// Phase Transition Logic
// ============================================================================

const PHASE_ORDER: readonly PipelinePhase[] = [
  "Upload",
  "PdfConversion",
  "Chunking",
  "Embedding",
  "Extraction",
  "GraphStorage",
] as const;

/**
 * WHY: Get the index of a phase in the pipeline
 * Used for ordering and transition validation
 */
function getPhaseIndex(phase: PipelinePhase): number {
  return PHASE_ORDER.indexOf(phase);
}

/**
 * WHY: Check if a phase transition is valid
 * Rules:
 * - pending → active: always allowed
 * - active → complete: allowed
 * - active → failed: allowed
 * - complete → active: not allowed (no going back)
 * - failed → active: allowed (retry)
 */
function isValidTransition(from: PhaseStatus, to: PhaseStatus): boolean {
  const transitions: Record<PhaseStatus, PhaseStatus[]> = {
    pending: ["active"],
    active: ["complete", "failed"],
    complete: [], // Cannot go back
    failed: ["active", "pending"], // Can retry
  };

  return transitions[from].includes(to);
}

/**
 * WHY: Check if a phase can start based on previous phases
 * A phase can only start if all previous phases are complete
 */
function canPhaseStart(phase: PipelinePhase, phases: PhaseState[]): boolean {
  const phaseIndex = getPhaseIndex(phase);

  // First phase can always start
  if (phaseIndex === 0) return true;

  // All previous phases must be complete
  for (let i = 0; i < phaseIndex; i++) {
    const prevPhase = phases.find((p) => p.phase === PHASE_ORDER[i]);
    if (!prevPhase || prevPhase.status !== "complete") {
      return false;
    }
  }

  return true;
}

/**
 * WHY: Get the next phase after a given phase
 */
function getNextPhase(phase: PipelinePhase): PipelinePhase | null {
  const index = getPhaseIndex(phase);
  if (index === -1 || index >= PHASE_ORDER.length - 1) {
    return null;
  }
  return PHASE_ORDER[index + 1];
}

/**
 * WHY: Get the previous phase before a given phase
 */
function getPreviousPhase(phase: PipelinePhase): PipelinePhase | null {
  const index = getPhaseIndex(phase);
  if (index <= 0) {
    return null;
  }
  return PHASE_ORDER[index - 1];
}

/**
 * WHY: Check if pipeline is complete
 * All phases must have status 'complete'
 */
function isPipelineComplete(phases: PhaseState[]): boolean {
  return (
    phases.length === PHASE_ORDER.length &&
    phases.every((p) => p.status === "complete")
  );
}

/**
 * WHY: Check if pipeline has failed
 * Any phase with status 'failed'
 */
function isPipelineFailed(phases: PhaseState[]): boolean {
  return phases.some((p) => p.status === "failed");
}

/**
 * WHY: Get the current active phase
 */
function getActivePhase(phases: PhaseState[]): PipelinePhase | null {
  const active = phases.find((p) => p.status === "active");
  return active ? active.phase : null;
}

/**
 * WHY: Calculate overall progress percentage
 * Each complete phase contributes 100/6 = 16.67%
 * Active phase contributes proportionally
 */
function calculatePipelineProgress(
  phases: PhaseState[],
  activePhasePercent: number = 0,
): number {
  const phaseWeight = 100 / PHASE_ORDER.length;
  let progress = 0;

  for (const state of phases) {
    if (state.status === "complete") {
      progress += phaseWeight;
    } else if (state.status === "active") {
      progress += (phaseWeight * activePhasePercent) / 100;
    }
  }

  return Math.round(progress * 100) / 100;
}

// ============================================================================
// Tests
// ============================================================================

describe("Phase Ordering", () => {
  describe("getPhaseIndex", () => {
    it("returns correct index for each phase", () => {
      expect(getPhaseIndex("Upload")).toBe(0);
      expect(getPhaseIndex("PdfConversion")).toBe(1);
      expect(getPhaseIndex("Chunking")).toBe(2);
      expect(getPhaseIndex("Embedding")).toBe(3);
      expect(getPhaseIndex("Extraction")).toBe(4);
      expect(getPhaseIndex("GraphStorage")).toBe(5);
    });
  });

  describe("getNextPhase", () => {
    it("returns next phase in sequence", () => {
      expect(getNextPhase("Upload")).toBe("PdfConversion");
      expect(getNextPhase("PdfConversion")).toBe("Chunking");
      expect(getNextPhase("Chunking")).toBe("Embedding");
      expect(getNextPhase("Embedding")).toBe("Extraction");
      expect(getNextPhase("Extraction")).toBe("GraphStorage");
    });

    it("returns null for last phase", () => {
      expect(getNextPhase("GraphStorage")).toBeNull();
    });
  });

  describe("getPreviousPhase", () => {
    it("returns previous phase in sequence", () => {
      expect(getPreviousPhase("GraphStorage")).toBe("Extraction");
      expect(getPreviousPhase("Extraction")).toBe("Embedding");
      expect(getPreviousPhase("Embedding")).toBe("Chunking");
      expect(getPreviousPhase("Chunking")).toBe("PdfConversion");
      expect(getPreviousPhase("PdfConversion")).toBe("Upload");
    });

    it("returns null for first phase", () => {
      expect(getPreviousPhase("Upload")).toBeNull();
    });
  });
});

describe("Phase Transitions", () => {
  describe("isValidTransition", () => {
    it("allows pending → active", () => {
      expect(isValidTransition("pending", "active")).toBe(true);
    });

    it("allows active → complete", () => {
      expect(isValidTransition("active", "complete")).toBe(true);
    });

    it("allows active → failed", () => {
      expect(isValidTransition("active", "failed")).toBe(true);
    });

    it("allows failed → active (retry)", () => {
      expect(isValidTransition("failed", "active")).toBe(true);
    });

    it("allows failed → pending (reset)", () => {
      expect(isValidTransition("failed", "pending")).toBe(true);
    });

    it("disallows complete → any", () => {
      expect(isValidTransition("complete", "active")).toBe(false);
      expect(isValidTransition("complete", "pending")).toBe(false);
      expect(isValidTransition("complete", "failed")).toBe(false);
    });

    it("disallows pending → complete (must go through active)", () => {
      expect(isValidTransition("pending", "complete")).toBe(false);
    });

    it("disallows pending → failed", () => {
      expect(isValidTransition("pending", "failed")).toBe(false);
    });
  });

  describe("canPhaseStart", () => {
    it("allows first phase to start always", () => {
      const emptyPhases: PhaseState[] = [];
      expect(canPhaseStart("Upload", emptyPhases)).toBe(true);
    });

    it("requires previous phase to be complete", () => {
      const phases: PhaseState[] = [{ phase: "Upload", status: "complete" }];
      expect(canPhaseStart("PdfConversion", phases)).toBe(true);
    });

    it("blocks if previous phase not complete", () => {
      const phases: PhaseState[] = [{ phase: "Upload", status: "active" }];
      expect(canPhaseStart("PdfConversion", phases)).toBe(false);
    });

    it("requires all previous phases complete", () => {
      const phases: PhaseState[] = [
        { phase: "Upload", status: "complete" },
        { phase: "PdfConversion", status: "complete" },
        { phase: "Chunking", status: "active" }, // Not complete
      ];
      expect(canPhaseStart("Embedding", phases)).toBe(false);
    });

    it("allows starting middle phase if all prior complete", () => {
      const phases: PhaseState[] = [
        { phase: "Upload", status: "complete" },
        { phase: "PdfConversion", status: "complete" },
        { phase: "Chunking", status: "complete" },
      ];
      expect(canPhaseStart("Embedding", phases)).toBe(true);
    });
  });
});

describe("Pipeline Status", () => {
  describe("isPipelineComplete", () => {
    it("returns false for empty phases", () => {
      expect(isPipelineComplete([])).toBe(false);
    });

    it("returns false if any phase not complete", () => {
      const phases: PhaseState[] = PHASE_ORDER.map((phase) => ({
        phase,
        status: phase === "GraphStorage" ? "active" : "complete",
      }));
      expect(isPipelineComplete(phases)).toBe(false);
    });

    it("returns true if all phases complete", () => {
      const phases: PhaseState[] = PHASE_ORDER.map((phase) => ({
        phase,
        status: "complete" as PhaseStatus,
      }));
      expect(isPipelineComplete(phases)).toBe(true);
    });
  });

  describe("isPipelineFailed", () => {
    it("returns false for no failures", () => {
      const phases: PhaseState[] = [
        { phase: "Upload", status: "complete" },
        { phase: "PdfConversion", status: "active" },
      ];
      expect(isPipelineFailed(phases)).toBe(false);
    });

    it("returns true if any phase failed", () => {
      const phases: PhaseState[] = [
        { phase: "Upload", status: "complete" },
        { phase: "PdfConversion", status: "failed" },
      ];
      expect(isPipelineFailed(phases)).toBe(true);
    });
  });

  describe("getActivePhase", () => {
    it("returns null if no active phase", () => {
      const phases: PhaseState[] = [
        { phase: "Upload", status: "complete" },
        { phase: "PdfConversion", status: "pending" },
      ];
      expect(getActivePhase(phases)).toBeNull();
    });

    it("returns the active phase", () => {
      const phases: PhaseState[] = [
        { phase: "Upload", status: "complete" },
        { phase: "PdfConversion", status: "active" },
      ];
      expect(getActivePhase(phases)).toBe("PdfConversion");
    });
  });
});

describe("Pipeline Progress Calculation", () => {
  describe("calculatePipelineProgress", () => {
    it("returns 0 for no progress", () => {
      const phases: PhaseState[] = PHASE_ORDER.map((phase) => ({
        phase,
        status: "pending" as PhaseStatus,
      }));
      expect(calculatePipelineProgress(phases)).toBe(0);
    });

    it("returns ~16.67% for one complete phase", () => {
      const phases: PhaseState[] = PHASE_ORDER.map((phase, i) => ({
        phase,
        status: (i === 0 ? "complete" : "pending") as PhaseStatus,
      }));
      expect(calculatePipelineProgress(phases)).toBeCloseTo(16.67, 1);
    });

    it("returns 50% for three complete phases", () => {
      const phases: PhaseState[] = PHASE_ORDER.map((phase, i) => ({
        phase,
        status: (i < 3 ? "complete" : "pending") as PhaseStatus,
      }));
      expect(calculatePipelineProgress(phases)).toBe(50);
    });

    it("returns 100% for all complete", () => {
      const phases: PhaseState[] = PHASE_ORDER.map((phase) => ({
        phase,
        status: "complete" as PhaseStatus,
      }));
      expect(calculatePipelineProgress(phases)).toBe(100);
    });

    it("includes active phase partial progress", () => {
      const phases: PhaseState[] = [
        { phase: "Upload", status: "complete" },
        { phase: "PdfConversion", status: "active" },
        { phase: "Chunking", status: "pending" },
        { phase: "Embedding", status: "pending" },
        { phase: "Extraction", status: "pending" },
        { phase: "GraphStorage", status: "pending" },
      ];
      // 1 complete (16.67) + 50% of active (8.33) = 25
      expect(calculatePipelineProgress(phases, 50)).toBeCloseTo(25, 0);
    });
  });
});

describe("Complex Scenarios", () => {
  describe("retry after failure", () => {
    it("can transition back to active state", () => {
      // Failed state
      const failedPhase: PhaseStatus = "failed";

      // Can retry
      expect(isValidTransition(failedPhase, "active")).toBe(true);
    });

    it("resets to pending is allowed", () => {
      expect(isValidTransition("failed", "pending")).toBe(true);
    });
  });

  describe("phase ordering invariant", () => {
    it("maintains correct order", () => {
      for (let i = 0; i < PHASE_ORDER.length - 1; i++) {
        const next = getNextPhase(PHASE_ORDER[i]);
        expect(next).toBe(PHASE_ORDER[i + 1]);
      }
    });

    it("has exactly 6 phases", () => {
      expect(PHASE_ORDER.length).toBe(6);
    });
  });
});
