/**
 * OODA-47: Cost Badge Logic Unit Tests
 *
 * @description Tests for cost formatting and display logic
 * @implements Phase 4 Testing & Validation
 * @see specs/001-upload-pdf.md
 */

import type { CostBreakdown, StageCostBreakdown } from "@/types/cost";
import { describe, expect, it } from "vitest";

// ============================================================================
// Helper Functions (replicate logic from cost-badge.tsx)
// ============================================================================

/**
 * Formats cost as USD string.
 */
function formatCost(cost: number): string {
  if (cost === 0) return "$0.00";
  if (cost < 0.0001) return "<$0.0001";
  if (cost < 0.01) return `$${cost.toFixed(4)}`;
  if (cost < 1) return `$${cost.toFixed(3)}`;
  return `$${cost.toFixed(2)}`;
}

/**
 * Formats token count with K/M suffix.
 */
function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}K`;
  return tokens.toString();
}

/**
 * Calculate cost trend percentage.
 */
function calculateTrend(cost: number, estimated?: number): number | null {
  if (!estimated || cost <= 0) return null;
  return ((cost - estimated) / estimated) * 100;
}

/**
 * Determine trend direction.
 */
function getTrendDirection(trend: number | null): "up" | "down" | "neutral" {
  if (trend === null) return "neutral";
  if (trend > 5) return "up";
  if (trend < -5) return "down";
  return "neutral";
}

/**
 * Get trend color based on percentage.
 */
function getTrendColor(trend: number | null): string {
  if (trend === null) return "text-muted-foreground";
  if (trend > 10) return "text-red-500";
  if (trend < -10) return "text-green-500";
  return "text-muted-foreground";
}

/**
 * Calculate total cost from breakdown.
 */
function calculateTotalFromBreakdown(breakdown: CostBreakdown): number {
  if (breakdown.by_stage) {
    return breakdown.by_stage.reduce(
      (sum: number, stage: StageCostBreakdown) => sum + stage.cost,
      0,
    );
  }
  return breakdown.total_cost;
}

// ============================================================================
// formatCost Tests
// ============================================================================

describe("formatCost", () => {
  describe("zero value", () => {
    it("should format 0 as $0.00", () => {
      expect(formatCost(0)).toBe("$0.00");
    });
  });

  describe("very small values", () => {
    it("should show <$0.0001 for tiny values", () => {
      expect(formatCost(0.00001)).toBe("<$0.0001");
      expect(formatCost(0.00005)).toBe("<$0.0001");
      expect(formatCost(0.00009)).toBe("<$0.0001");
    });

    it("should format small values with 4 decimals", () => {
      expect(formatCost(0.0001)).toBe("$0.0001");
      expect(formatCost(0.0005)).toBe("$0.0005");
      expect(formatCost(0.001)).toBe("$0.0010");
      expect(formatCost(0.0099)).toBe("$0.0099");
    });
  });

  describe("cent-level values", () => {
    it("should format values under $1 with 3 decimals", () => {
      expect(formatCost(0.01)).toBe("$0.010");
      expect(formatCost(0.05)).toBe("$0.050");
      expect(formatCost(0.1)).toBe("$0.100");
      expect(formatCost(0.25)).toBe("$0.250");
      expect(formatCost(0.99)).toBe("$0.990");
    });
  });

  describe("dollar-level values", () => {
    it("should format $1+ with 2 decimals", () => {
      expect(formatCost(1)).toBe("$1.00");
      expect(formatCost(1.5)).toBe("$1.50");
      expect(formatCost(10)).toBe("$10.00");
      expect(formatCost(99.99)).toBe("$99.99");
      expect(formatCost(1000)).toBe("$1000.00");
    });
  });

  describe("edge cases", () => {
    it("should handle boundary at 0.0001", () => {
      expect(formatCost(0.00009999)).toBe("<$0.0001");
      expect(formatCost(0.0001)).toBe("$0.0001");
    });

    it("should handle boundary at 0.01", () => {
      expect(formatCost(0.0099)).toBe("$0.0099");
      expect(formatCost(0.01)).toBe("$0.010");
    });

    it("should handle boundary at 1", () => {
      expect(formatCost(0.999)).toBe("$0.999");
      expect(formatCost(1.0)).toBe("$1.00");
    });

    it("should handle negative values", () => {
      // Negative costs are treated as very small values due to the < 0.0001 check
      // In practice, negative costs shouldn't occur
      expect(formatCost(-1)).toBe("<$0.0001");
    });
  });
});

// ============================================================================
// formatTokens Tests
// ============================================================================

describe("formatTokens", () => {
  describe("small values", () => {
    it("should format values under 1K without suffix", () => {
      expect(formatTokens(0)).toBe("0");
      expect(formatTokens(1)).toBe("1");
      expect(formatTokens(100)).toBe("100");
      expect(formatTokens(999)).toBe("999");
    });
  });

  describe("thousands", () => {
    it("should format with K suffix", () => {
      expect(formatTokens(1000)).toBe("1.0K");
      expect(formatTokens(1500)).toBe("1.5K");
      expect(formatTokens(10000)).toBe("10.0K");
      expect(formatTokens(999999)).toBe("1000.0K");
    });

    it("should show one decimal place", () => {
      expect(formatTokens(1234)).toBe("1.2K");
      expect(formatTokens(5678)).toBe("5.7K");
    });
  });

  describe("millions", () => {
    it("should format with M suffix", () => {
      expect(formatTokens(1_000_000)).toBe("1.0M");
      expect(formatTokens(1_500_000)).toBe("1.5M");
      expect(formatTokens(10_000_000)).toBe("10.0M");
    });

    it("should show one decimal place", () => {
      expect(formatTokens(1_234_567)).toBe("1.2M");
      expect(formatTokens(5_678_900)).toBe("5.7M");
    });
  });

  describe("boundary values", () => {
    it("should handle boundary at 1000", () => {
      expect(formatTokens(999)).toBe("999");
      expect(formatTokens(1000)).toBe("1.0K");
    });

    it("should handle boundary at 1M", () => {
      expect(formatTokens(999999)).toBe("1000.0K");
      expect(formatTokens(1000000)).toBe("1.0M");
    });
  });
});

// ============================================================================
// calculateTrend Tests
// ============================================================================

describe("calculateTrend", () => {
  it("should return null when no estimate", () => {
    expect(calculateTrend(10, undefined)).toBeNull();
  });

  it("should return null when cost is 0", () => {
    expect(calculateTrend(0, 10)).toBeNull();
  });

  it("should return null when cost is negative", () => {
    expect(calculateTrend(-1, 10)).toBeNull();
  });

  it("should calculate positive trend (over budget)", () => {
    // Cost $15, Estimate $10 = 50% over
    expect(calculateTrend(15, 10)).toBe(50);
  });

  it("should calculate negative trend (under budget)", () => {
    // Cost $5, Estimate $10 = -50% (under budget)
    expect(calculateTrend(5, 10)).toBe(-50);
  });

  it("should calculate 0% trend when on budget", () => {
    expect(calculateTrend(10, 10)).toBe(0);
  });

  it("should handle small differences", () => {
    // Cost $10.50, Estimate $10 = 5%
    expect(calculateTrend(10.5, 10)).toBe(5);
  });
});

// ============================================================================
// getTrendDirection Tests
// ============================================================================

describe("getTrendDirection", () => {
  it("should return neutral for null", () => {
    expect(getTrendDirection(null)).toBe("neutral");
  });

  it("should return neutral for small positive trends", () => {
    expect(getTrendDirection(0)).toBe("neutral");
    expect(getTrendDirection(3)).toBe("neutral");
    expect(getTrendDirection(5)).toBe("neutral");
  });

  it("should return neutral for small negative trends", () => {
    expect(getTrendDirection(-3)).toBe("neutral");
    expect(getTrendDirection(-5)).toBe("neutral");
  });

  it("should return up for significant positive trends", () => {
    expect(getTrendDirection(6)).toBe("up");
    expect(getTrendDirection(10)).toBe("up");
    expect(getTrendDirection(100)).toBe("up");
  });

  it("should return down for significant negative trends", () => {
    expect(getTrendDirection(-6)).toBe("down");
    expect(getTrendDirection(-10)).toBe("down");
    expect(getTrendDirection(-50)).toBe("down");
  });
});

// ============================================================================
// getTrendColor Tests
// ============================================================================

describe("getTrendColor", () => {
  it("should return muted for null", () => {
    expect(getTrendColor(null)).toBe("text-muted-foreground");
  });

  it("should return red for significantly over budget", () => {
    expect(getTrendColor(11)).toBe("text-red-500");
    expect(getTrendColor(50)).toBe("text-red-500");
  });

  it("should return green for significantly under budget", () => {
    expect(getTrendColor(-11)).toBe("text-green-500");
    expect(getTrendColor(-50)).toBe("text-green-500");
  });

  it("should return muted for small variations", () => {
    expect(getTrendColor(0)).toBe("text-muted-foreground");
    expect(getTrendColor(5)).toBe("text-muted-foreground");
    expect(getTrendColor(-5)).toBe("text-muted-foreground");
    expect(getTrendColor(10)).toBe("text-muted-foreground");
    expect(getTrendColor(-10)).toBe("text-muted-foreground");
  });

  it("should handle boundary at +/-10", () => {
    expect(getTrendColor(10)).toBe("text-muted-foreground");
    expect(getTrendColor(10.01)).toBe("text-red-500");
    expect(getTrendColor(-10)).toBe("text-muted-foreground");
    expect(getTrendColor(-10.01)).toBe("text-green-500");
  });
});

// ============================================================================
// calculateTotalFromBreakdown Tests
// ============================================================================

describe("calculateTotalFromBreakdown", () => {
  it("should return total_cost when no stages", () => {
    const breakdown: CostBreakdown = {
      total_cost: 1.5,
    };
    expect(calculateTotalFromBreakdown(breakdown)).toBe(1.5);
  });

  it("should sum stage costs when available", () => {
    const breakdown: CostBreakdown = {
      total_cost: 2.0,
      by_stage: [
        {
          stage: "preprocessing",
          cost: 0.1,
          tokens: { input: 100, output: 50, total: 150 },
        },
        {
          stage: "extracting",
          cost: 1.5,
          tokens: { input: 10000, output: 5000, total: 15000 },
        },
        {
          stage: "embedding",
          cost: 0.4,
          tokens: { input: 5000, output: 0, total: 5000 },
        },
      ],
    };
    expect(calculateTotalFromBreakdown(breakdown)).toBe(2.0);
  });

  it("should handle empty stages array", () => {
    const breakdown: CostBreakdown = {
      total_cost: 0.5,
      by_stage: [],
    };
    expect(calculateTotalFromBreakdown(breakdown)).toBe(0);
  });

  it("should handle single stage", () => {
    const breakdown: CostBreakdown = {
      total_cost: 0.25,
      by_stage: [
        {
          stage: "indexing",
          cost: 0.25,
          tokens: { input: 500, output: 100, total: 600 },
        },
      ],
    };
    expect(calculateTotalFromBreakdown(breakdown)).toBe(0.25);
  });
});

// ============================================================================
// Integration Tests
// ============================================================================

describe("Integration", () => {
  describe("realistic cost scenarios", () => {
    it("should format typical PDF processing cost", () => {
      // Typical PDF: ~$0.05-0.15
      expect(formatCost(0.08)).toBe("$0.080");
      expect(formatCost(0.12)).toBe("$0.120");
    });

    it("should format large batch processing cost", () => {
      // Large batch: ~$5-50
      expect(formatCost(15.75)).toBe("$15.75");
      expect(formatCost(42.0)).toBe("$42.00");
    });

    it("should format token counts for typical extraction", () => {
      // Input tokens (document content)
      expect(formatTokens(25000)).toBe("25.0K");
      // Output tokens (entities, relationships)
      expect(formatTokens(3500)).toBe("3.5K");
    });
  });

  describe("cost breakdown formatting", () => {
    it("should display full breakdown correctly", () => {
      const breakdown: CostBreakdown = {
        total_cost: 0.145,
        by_stage: [
          {
            stage: "preprocessing",
            cost: 0.005,
            tokens: { input: 500, output: 100, total: 600 },
          },
          {
            stage: "chunking",
            cost: 0.01,
            tokens: { input: 1000, output: 200, total: 1200 },
          },
          {
            stage: "extracting",
            cost: 0.1,
            tokens: { input: 10000, output: 5000, total: 15000 },
          },
          {
            stage: "embedding",
            cost: 0.03,
            tokens: { input: 3000, output: 0, total: 3000 },
          },
        ],
        tokens: {
          input: 14500,
          output: 5300,
          total: 19800,
        },
      };

      // Verify formatting of each part
      expect(formatCost(breakdown.total_cost)).toBe("$0.145");

      breakdown.by_stage?.forEach((stage: StageCostBreakdown) => {
        const formatted = formatCost(stage.cost);
        expect(formatted).toMatch(/^\$\d+\.\d+$/);
      });

      if (breakdown.tokens) {
        expect(formatTokens(breakdown.tokens.input)).toBe("14.5K");
        expect(formatTokens(breakdown.tokens.output)).toBe("5.3K");
        expect(formatTokens(breakdown.tokens.total)).toBe("19.8K");
      }
    });
  });

  describe("trend analysis", () => {
    it("should identify over-budget scenario", () => {
      const cost = 0.25;
      const estimated = 0.15;
      const trend = calculateTrend(cost, estimated);

      expect(trend).toBeCloseTo(66.67, 1);
      expect(getTrendDirection(trend)).toBe("up");
      expect(getTrendColor(trend)).toBe("text-red-500");
    });

    it("should identify under-budget scenario", () => {
      const cost = 0.1;
      const estimated = 0.2;
      const trend = calculateTrend(cost, estimated);

      expect(trend).toBe(-50);
      expect(getTrendDirection(trend)).toBe("down");
      expect(getTrendColor(trend)).toBe("text-green-500");
    });

    it("should identify on-budget scenario", () => {
      const cost = 0.15;
      const estimated = 0.15;
      const trend = calculateTrend(cost, estimated);

      expect(trend).toBe(0);
      expect(getTrendDirection(trend)).toBe("neutral");
      expect(getTrendColor(trend)).toBe("text-muted-foreground");
    });
  });
});

// ============================================================================
// Edge Cases
// ============================================================================

describe("Edge Cases", () => {
  describe("extreme values", () => {
    it("should handle very large costs", () => {
      expect(formatCost(10000)).toBe("$10000.00");
      expect(formatCost(1000000)).toBe("$1000000.00");
    });

    it("should handle very large token counts", () => {
      expect(formatTokens(100_000_000)).toBe("100.0M");
      expect(formatTokens(1_000_000_000)).toBe("1000.0M");
    });

    it("should handle very small costs", () => {
      expect(formatCost(0.000001)).toBe("<$0.0001");
    });
  });

  describe("floating point precision", () => {
    it("should handle floating point edge cases", () => {
      // 0.1 + 0.2 = 0.30000000000000004 in JS
      const cost = 0.1 + 0.2;
      const formatted = formatCost(cost);
      expect(formatted).toBe("$0.300");
    });

    it("should handle rounding correctly", () => {
      expect(formatCost(1.999)).toBe("$2.00");
      // 0.9995 is < 1, so it uses 3 decimal format
      expect(formatCost(0.9995)).toBe("$1.000");
    });
  });

  describe("negative values", () => {
    it("should handle negative token counts", () => {
      // Shouldn't happen but should handle gracefully
      expect(formatTokens(-100)).toBe("-100");
    });

    it("should handle negative trends", () => {
      expect(getTrendDirection(-100)).toBe("down");
      expect(getTrendColor(-100)).toBe("text-green-500");
    });
  });
});

// ============================================================================
// Performance Tests
// ============================================================================

describe("Performance", () => {
  it("should format costs quickly", () => {
    const start = performance.now();

    for (let i = 0; i < 10000; i++) {
      formatCost(Math.random() * 100);
    }

    const duration = performance.now() - start;
    expect(duration).toBeLessThan(50);
  });

  it("should format tokens quickly", () => {
    const start = performance.now();

    for (let i = 0; i < 10000; i++) {
      formatTokens(Math.floor(Math.random() * 10_000_000));
    }

    const duration = performance.now() - start;
    expect(duration).toBeLessThan(50);
  });

  it("should calculate trends quickly", () => {
    const start = performance.now();

    for (let i = 0; i < 10000; i++) {
      const cost = Math.random() * 10;
      const estimated = Math.random() * 10;
      calculateTrend(cost, estimated);
    }

    const duration = performance.now() - start;
    expect(duration).toBeLessThan(20);
  });
});
