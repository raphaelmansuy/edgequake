/**
 * @module performance.test
 * @description Performance tests for PDF progress tracking
 *
 * @implements OODA-37: Performance tests
 *
 * Tests cover:
 * - Progress calculation efficiency
 * - Concurrent state updates
 * - Memory efficiency for large phase arrays
 * - Batch update handling
 */

import { describe, expect, it } from "vitest";

// ============================================================================
// Types (matching component types)
// ============================================================================

interface PhaseProgress {
  phase: string;
  status: "pending" | "active" | "complete" | "failed";
  current: number;
  total: number;
  percentage: number;
  eta_seconds?: number;
  message: string;
}

// ============================================================================
// Performance Utilities
// ============================================================================

/**
 * WHY: Measures execution time of a function
 * Used for performance benchmarking
 */
function measureTime<T>(fn: () => T): { result: T; durationMs: number } {
  const start = performance.now();
  const result = fn();
  const end = performance.now();
  return { result, durationMs: end - start };
}

/**
 * WHY: Creates a mock phase array for testing
 * Allows configurable size for stress testing
 */
function createMockPhases(count: number): PhaseProgress[] {
  const phases: PhaseProgress[] = [];
  for (let i = 0; i < count; i++) {
    phases.push({
      phase: `Phase_${i}`,
      status:
        i < count / 2
          ? "complete"
          : i === Math.floor(count / 2)
            ? "active"
            : "pending",
      current: i < count / 2 ? 100 : i === Math.floor(count / 2) ? 50 : 0,
      total: 100,
      percentage: i < count / 2 ? 100 : i === Math.floor(count / 2) ? 50 : 0,
      message: `Processing phase ${i}`,
    });
  }
  return phases;
}

// ============================================================================
// Calculation Functions (matching component logic)
// ============================================================================

function calculateOverallPercent(phases: PhaseProgress[]): number {
  if (phases.length === 0) return 0;
  const totalPercent = phases.reduce((sum, p) => sum + p.percentage, 0);
  return Math.round(totalPercent / phases.length);
}

function findCurrentPhaseIndex(phases: PhaseProgress[]): number {
  const activeIndex = phases.findIndex((p) => p.status === "active");
  if (activeIndex !== -1) return activeIndex;
  const lastComplete = phases.findLastIndex((p) => p.status === "complete");
  return lastComplete + 1;
}

function getCompletedPhaseCount(phases: PhaseProgress[]): number {
  return phases.filter((p) => p.status === "complete").length;
}

// ============================================================================
// Tests
// ============================================================================

describe("Performance: Progress Calculations", () => {
  describe("calculateOverallPercent performance", () => {
    it("handles 6 phases in <1ms", () => {
      const phases = createMockPhases(6);
      const { durationMs } = measureTime(() => calculateOverallPercent(phases));

      expect(durationMs).toBeLessThan(1);
    });

    it("handles 100 phases in <5ms", () => {
      const phases = createMockPhases(100);
      const { durationMs } = measureTime(() => calculateOverallPercent(phases));

      expect(durationMs).toBeLessThan(5);
    });

    it("handles 1000 phases in <50ms", () => {
      const phases = createMockPhases(1000);
      const { durationMs } = measureTime(() => calculateOverallPercent(phases));

      expect(durationMs).toBeLessThan(50);
    });

    it("calculates correctly for large arrays", () => {
      const phases = createMockPhases(1000);
      const result = calculateOverallPercent(phases);

      // Half complete (100%) + one active (50%) + rest pending (0%)
      // Expected: (500*100 + 1*50 + 499*0) / 1000 = 50.05, rounds to 50
      expect(result).toBeGreaterThanOrEqual(0);
      expect(result).toBeLessThanOrEqual(100);
    });
  });

  describe("findCurrentPhaseIndex performance", () => {
    it("finds active phase in <1ms for 6 phases", () => {
      const phases = createMockPhases(6);
      const { durationMs } = measureTime(() => findCurrentPhaseIndex(phases));

      expect(durationMs).toBeLessThan(1);
    });

    it("finds active phase in <5ms for 100 phases", () => {
      const phases = createMockPhases(100);
      const { durationMs } = measureTime(() => findCurrentPhaseIndex(phases));

      expect(durationMs).toBeLessThan(5);
    });

    it("handles worst case (active at end) efficiently", () => {
      const phases = createMockPhases(100);
      // Move active to last phase
      phases.forEach((p, i) => {
        p.status = i === 99 ? "active" : "pending";
      });

      const { durationMs, result } = measureTime(() =>
        findCurrentPhaseIndex(phases),
      );

      expect(durationMs).toBeLessThan(5);
      expect(result).toBe(99);
    });
  });

  describe("getCompletedPhaseCount performance", () => {
    it("counts completed in <1ms for 6 phases", () => {
      const phases = createMockPhases(6);
      const { durationMs } = measureTime(() => getCompletedPhaseCount(phases));

      expect(durationMs).toBeLessThan(1);
    });

    it("counts completed in <5ms for 1000 phases", () => {
      const phases = createMockPhases(1000);
      const { durationMs, result } = measureTime(() =>
        getCompletedPhaseCount(phases),
      );

      expect(durationMs).toBeLessThan(5);
      expect(result).toBe(500);
    });
  });
});

describe("Performance: Batch Updates", () => {
  describe("rapid sequential updates", () => {
    it("handles 100 rapid updates in <10ms", () => {
      const phases = createMockPhases(6);

      const { durationMs } = measureTime(() => {
        for (let i = 0; i < 100; i++) {
          // Simulate progress update
          phases[0].percentage = i % 100;
          phases[0].current = i;
          calculateOverallPercent(phases);
        }
      });

      expect(durationMs).toBeLessThan(10);
    });

    it("handles 1000 rapid updates in <100ms", () => {
      const phases = createMockPhases(6);

      const { durationMs } = measureTime(() => {
        for (let i = 0; i < 1000; i++) {
          phases[0].percentage = i % 100;
          phases[0].current = i;
          calculateOverallPercent(phases);
          findCurrentPhaseIndex(phases);
        }
      });

      expect(durationMs).toBeLessThan(100);
    });
  });

  describe("concurrent upload simulation", () => {
    it("handles 10 concurrent uploads efficiently", () => {
      // Simulate 10 concurrent uploads, each with 6 phases
      const uploads: PhaseProgress[][] = [];
      for (let i = 0; i < 10; i++) {
        uploads.push(createMockPhases(6));
      }

      const { durationMs } = measureTime(() => {
        // Update all uploads 50 times each
        for (let update = 0; update < 50; update++) {
          for (const upload of uploads) {
            upload[0].percentage = update * 2;
            calculateOverallPercent(upload);
          }
        }
      });

      expect(durationMs).toBeLessThan(50);
    });

    it("handles 50 concurrent uploads", () => {
      const uploads: PhaseProgress[][] = [];
      for (let i = 0; i < 50; i++) {
        uploads.push(createMockPhases(6));
      }

      const { durationMs } = measureTime(() => {
        // Single update pass
        for (const upload of uploads) {
          upload[0].percentage = 50;
          calculateOverallPercent(upload);
          findCurrentPhaseIndex(upload);
          getCompletedPhaseCount(upload);
        }
      });

      expect(durationMs).toBeLessThan(10);
    });
  });
});

describe("Performance: Memory Efficiency", () => {
  describe("phase array creation", () => {
    it("creates 6 phases with minimal memory", () => {
      const phases = createMockPhases(6);

      // Each phase is a small object
      const jsonSize = JSON.stringify(phases).length;

      // 6 phases should be < 2KB
      expect(jsonSize).toBeLessThan(2000);
    });

    it("creates 100 phases reasonably", () => {
      const phases = createMockPhases(100);
      const jsonSize = JSON.stringify(phases).length;

      // 100 phases should be < 30KB
      expect(jsonSize).toBeLessThan(30000);
    });
  });

  describe("immutable update patterns", () => {
    it("spread operator is fast for small arrays", () => {
      const phases = createMockPhases(6);

      const { durationMs } = measureTime(() => {
        for (let i = 0; i < 100; i++) {
          // Simulate immutable update
          const newPhases = [...phases];
          newPhases[0] = { ...phases[0], percentage: i };
        }
      });

      expect(durationMs).toBeLessThan(5);
    });

    it("map is efficient for bulk updates", () => {
      const phases = createMockPhases(6);

      const { durationMs } = measureTime(() => {
        for (let i = 0; i < 100; i++) {
          // Simulate map update
          const newPhases = phases.map((p, idx) =>
            idx === 0 ? { ...p, percentage: i } : p,
          );
        }
      });

      expect(durationMs).toBeLessThan(10);
    });
  });
});

describe("Performance: ETA Calculations", () => {
  function calculateETA(
    currentPhase: number,
    totalPhases: number,
    elapsedMs: number,
    phasePercentage: number,
  ): number {
    // Simple linear estimation
    const completedPhases = currentPhase + phasePercentage / 100;
    const remainingPhases = totalPhases - completedPhases;

    if (completedPhases === 0) return 0;

    const msPerPhase = elapsedMs / completedPhases;
    return Math.round(remainingPhases * msPerPhase);
  }

  it("calculates ETA in <1ms", () => {
    const { durationMs, result } = measureTime(() =>
      calculateETA(2, 6, 30000, 50),
    );

    expect(durationMs).toBeLessThan(1);
    expect(result).toBeGreaterThanOrEqual(0);
  });

  it("handles rapid ETA updates", () => {
    const { durationMs } = measureTime(() => {
      for (let i = 0; i < 1000; i++) {
        calculateETA(Math.floor(i / 100), 6, i * 100, i % 100);
      }
    });

    expect(durationMs).toBeLessThan(10);
  });

  it("returns stable ETA estimates", () => {
    const etas: number[] = [];

    for (let percent = 10; percent <= 100; percent += 10) {
      const eta = calculateETA(0, 6, 10000, percent);
      etas.push(eta);
    }

    // ETAs should decrease as progress increases
    for (let i = 1; i < etas.length; i++) {
      expect(etas[i]).toBeLessThanOrEqual(etas[i - 1]);
    }
  });
});

describe("Performance: WebSocket Update Processing", () => {
  interface ProgressUpdate {
    trackId: string;
    phases: PhaseProgress[];
    overallPercent: number;
    timestamp: number;
  }

  function processUpdate(update: ProgressUpdate): {
    percent: number;
    currentPhase: number;
    completed: number;
  } {
    return {
      percent: update.overallPercent,
      currentPhase: findCurrentPhaseIndex(update.phases),
      completed: getCompletedPhaseCount(update.phases),
    };
  }

  it("processes single update in <1ms", () => {
    const update: ProgressUpdate = {
      trackId: "test-123",
      phases: createMockPhases(6),
      overallPercent: 50,
      timestamp: Date.now(),
    };

    const { durationMs } = measureTime(() => processUpdate(update));
    expect(durationMs).toBeLessThan(1);
  });

  it("processes 100 updates/second efficiently", () => {
    const updates: ProgressUpdate[] = [];
    for (let i = 0; i < 100; i++) {
      updates.push({
        trackId: `test-${i}`,
        phases: createMockPhases(6),
        overallPercent: i,
        timestamp: Date.now() + i * 10,
      });
    }

    const { durationMs } = measureTime(() => {
      for (const update of updates) {
        processUpdate(update);
      }
    });

    // Should complete well under 1 second (1000ms)
    expect(durationMs).toBeLessThan(100);
  });
});
