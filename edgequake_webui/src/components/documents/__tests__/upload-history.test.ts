/**
 * @module upload-history.test
 * @description Unit tests for UploadHistory component logic
 *
 * @implements OODA-35: UploadHistory tests
 *
 * Tests cover:
 * - History item transformation
 * - Filtering logic (all/success/failed)
 * - Search functionality
 * - Stats calculation (success rate)
 * - Duration formatting
 */

import { describe, expect, it } from "vitest";

// ============================================================================
// Types (matching upload-history.tsx)
// ============================================================================

type HistoryFilter = "all" | "success" | "failed";

interface HistoryItem {
  type: "success" | "failed";
  trackId: string;
  documentId?: string;
  documentName?: string;
  timestamp: Date;
  durationMs?: number;
  chunks?: number;
  entities?: number;
  relationships?: number;
  error?: string;
}

// ============================================================================
// Logic (extracted from component)
// ============================================================================

/**
 * WHY: Formats duration in human-readable format
 * - Sub-second: show milliseconds (e.g., "234ms")
 * - Seconds: show seconds with 1 decimal (e.g., "2.5s")
 * - Missing: show dash
 */
function formatDuration(ms?: number): string {
  if (!ms) return "-";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

/**
 * WHY: Calculates success rate as percentage
 * Returns 0 if no history items (avoid division by zero)
 */
function calculateSuccessRate(items: HistoryItem[]): number {
  if (items.length === 0) return 0;
  const successCount = items.filter((i) => i.type === "success").length;
  return Math.round((successCount / items.length) * 100);
}

/**
 * WHY: Filters history items by status
 * 'all' returns everything, 'success'/'failed' filters by type
 */
function filterByStatus(
  items: HistoryItem[],
  filter: HistoryFilter,
): HistoryItem[] {
  if (filter === "all") return items;
  return items.filter((item) => item.type === filter);
}

/**
 * WHY: Searches history items by trackId, documentId, or documentName
 * Case-insensitive substring matching
 */
function searchItems(items: HistoryItem[], query: string): HistoryItem[] {
  if (!query.trim()) return items;
  const lowerQuery = query.toLowerCase();
  return items.filter(
    (item) =>
      item.trackId.toLowerCase().includes(lowerQuery) ||
      item.documentId?.toLowerCase().includes(lowerQuery) ||
      item.documentName?.toLowerCase().includes(lowerQuery),
  );
}

/**
 * WHY: Sorts history items by timestamp (newest first) and limits count
 */
function sortAndLimit(items: HistoryItem[], maxItems: number): HistoryItem[] {
  return items
    .sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())
    .slice(0, maxItems);
}

/**
 * WHY: Counts items by type for stats display
 */
function countByType(items: HistoryItem[]): {
  success: number;
  failed: number;
} {
  return {
    success: items.filter((i) => i.type === "success").length,
    failed: items.filter((i) => i.type === "failed").length,
  };
}

// ============================================================================
// Tests
// ============================================================================

describe("formatDuration", () => {
  it("returns dash for undefined", () => {
    expect(formatDuration(undefined)).toBe("-");
  });

  it("returns dash for zero", () => {
    expect(formatDuration(0)).toBe("-");
  });

  it("formats sub-second as milliseconds", () => {
    expect(formatDuration(234)).toBe("234ms");
    expect(formatDuration(999)).toBe("999ms");
  });

  it("formats seconds with 1 decimal", () => {
    expect(formatDuration(1000)).toBe("1.0s");
    expect(formatDuration(2500)).toBe("2.5s");
    expect(formatDuration(10000)).toBe("10.0s");
  });

  it("handles boundary at 1000ms", () => {
    expect(formatDuration(999)).toBe("999ms");
    expect(formatDuration(1000)).toBe("1.0s");
  });
});

describe("calculateSuccessRate", () => {
  it("returns 0 for empty array", () => {
    expect(calculateSuccessRate([])).toBe(0);
  });

  it("returns 100 for all success", () => {
    const items: HistoryItem[] = [
      { type: "success", trackId: "1", timestamp: new Date() },
      { type: "success", trackId: "2", timestamp: new Date() },
    ];
    expect(calculateSuccessRate(items)).toBe(100);
  });

  it("returns 0 for all failed", () => {
    const items: HistoryItem[] = [
      { type: "failed", trackId: "1", timestamp: new Date() },
      { type: "failed", trackId: "2", timestamp: new Date() },
    ];
    expect(calculateSuccessRate(items)).toBe(0);
  });

  it("calculates mixed rate correctly", () => {
    const items: HistoryItem[] = [
      { type: "success", trackId: "1", timestamp: new Date() },
      { type: "failed", trackId: "2", timestamp: new Date() },
    ];
    expect(calculateSuccessRate(items)).toBe(50);
  });

  it("rounds to nearest integer", () => {
    const items: HistoryItem[] = [
      { type: "success", trackId: "1", timestamp: new Date() },
      { type: "success", trackId: "2", timestamp: new Date() },
      { type: "failed", trackId: "3", timestamp: new Date() },
    ];
    expect(calculateSuccessRate(items)).toBe(67); // 66.67 rounds to 67
  });
});

describe("filterByStatus", () => {
  const items: HistoryItem[] = [
    { type: "success", trackId: "1", timestamp: new Date() },
    { type: "failed", trackId: "2", timestamp: new Date() },
    { type: "success", trackId: "3", timestamp: new Date() },
  ];

  it('returns all items for "all" filter', () => {
    expect(filterByStatus(items, "all")).toHaveLength(3);
  });

  it('returns only success items for "success" filter', () => {
    const result = filterByStatus(items, "success");
    expect(result).toHaveLength(2);
    expect(result.every((i) => i.type === "success")).toBe(true);
  });

  it('returns only failed items for "failed" filter', () => {
    const result = filterByStatus(items, "failed");
    expect(result).toHaveLength(1);
    expect(result[0].type).toBe("failed");
  });
});

describe("searchItems", () => {
  const items: HistoryItem[] = [
    {
      type: "success",
      trackId: "abc-123",
      documentId: "doc-xyz",
      documentName: "Report.pdf",
      timestamp: new Date(),
    },
    {
      type: "success",
      trackId: "def-456",
      documentId: "doc-abc",
      documentName: "Invoice.pdf",
      timestamp: new Date(),
    },
    { type: "failed", trackId: "ghi-789", timestamp: new Date() },
  ];

  it("returns all items for empty query", () => {
    expect(searchItems(items, "")).toHaveLength(3);
    expect(searchItems(items, "   ")).toHaveLength(3);
  });

  it("searches by trackId", () => {
    const result = searchItems(items, "abc");
    expect(result).toHaveLength(2); // abc-123 and doc-abc
  });

  it("searches by documentId", () => {
    const result = searchItems(items, "xyz");
    expect(result).toHaveLength(1);
    expect(result[0].documentId).toBe("doc-xyz");
  });

  it("searches by documentName", () => {
    const result = searchItems(items, "report");
    expect(result).toHaveLength(1);
    expect(result[0].documentName).toBe("Report.pdf");
  });

  it("is case-insensitive", () => {
    expect(searchItems(items, "ABC")).toHaveLength(2);
    expect(searchItems(items, "REPORT")).toHaveLength(1);
  });

  it("returns empty for no matches", () => {
    expect(searchItems(items, "zzz")).toHaveLength(0);
  });
});

describe("sortAndLimit", () => {
  const now = new Date();
  const oneHourAgo = new Date(now.getTime() - 3600000);
  const twoHoursAgo = new Date(now.getTime() - 7200000);

  const items: HistoryItem[] = [
    { type: "success", trackId: "old", timestamp: twoHoursAgo },
    { type: "success", trackId: "new", timestamp: now },
    { type: "success", trackId: "mid", timestamp: oneHourAgo },
  ];

  it("sorts by timestamp descending (newest first)", () => {
    const result = sortAndLimit(items, 10);
    expect(result[0].trackId).toBe("new");
    expect(result[1].trackId).toBe("mid");
    expect(result[2].trackId).toBe("old");
  });

  it("limits to maxItems", () => {
    const result = sortAndLimit(items, 2);
    expect(result).toHaveLength(2);
    expect(result[0].trackId).toBe("new");
    expect(result[1].trackId).toBe("mid");
  });

  it("returns all if maxItems > items.length", () => {
    const result = sortAndLimit(items, 100);
    expect(result).toHaveLength(3);
  });

  it("handles empty array", () => {
    expect(sortAndLimit([], 10)).toHaveLength(0);
  });
});

describe("countByType", () => {
  it("returns zeros for empty array", () => {
    const result = countByType([]);
    expect(result).toEqual({ success: 0, failed: 0 });
  });

  it("counts success correctly", () => {
    const items: HistoryItem[] = [
      { type: "success", trackId: "1", timestamp: new Date() },
      { type: "success", trackId: "2", timestamp: new Date() },
    ];
    expect(countByType(items)).toEqual({ success: 2, failed: 0 });
  });

  it("counts failed correctly", () => {
    const items: HistoryItem[] = [
      { type: "failed", trackId: "1", timestamp: new Date() },
    ];
    expect(countByType(items)).toEqual({ success: 0, failed: 1 });
  });

  it("counts mixed correctly", () => {
    const items: HistoryItem[] = [
      { type: "success", trackId: "1", timestamp: new Date() },
      { type: "failed", trackId: "2", timestamp: new Date() },
      { type: "success", trackId: "3", timestamp: new Date() },
      { type: "failed", trackId: "4", timestamp: new Date() },
      { type: "failed", trackId: "5", timestamp: new Date() },
    ];
    expect(countByType(items)).toEqual({ success: 2, failed: 3 });
  });
});

describe("HistoryItem interface", () => {
  it("accepts minimal success item", () => {
    const item: HistoryItem = {
      type: "success",
      trackId: "track-123",
      timestamp: new Date(),
    };
    expect(item.type).toBe("success");
  });

  it("accepts minimal failed item", () => {
    const item: HistoryItem = {
      type: "failed",
      trackId: "track-456",
      timestamp: new Date(),
    };
    expect(item.type).toBe("failed");
  });

  it("accepts full item with all optional fields", () => {
    const item: HistoryItem = {
      type: "success",
      trackId: "track-789",
      documentId: "doc-001",
      documentName: "Annual Report.pdf",
      timestamp: new Date(),
      durationMs: 5432,
      chunks: 10,
      entities: 45,
      relationships: 23,
    };
    expect(item.entities).toBe(45);
    expect(item.relationships).toBe(23);
  });

  it("accepts failed item with error", () => {
    const item: HistoryItem = {
      type: "failed",
      trackId: "track-error",
      timestamp: new Date(),
      error: "PDF parsing failed at page 5",
    };
    expect(item.error).toContain("PDF parsing failed");
  });
});

describe("integration scenarios", () => {
  it("filters and searches combined", () => {
    const items: HistoryItem[] = [
      {
        type: "success",
        trackId: "report-abc",
        documentName: "Report.pdf",
        timestamp: new Date(),
      },
      {
        type: "failed",
        trackId: "report-def",
        documentName: "Report2.pdf",
        timestamp: new Date(),
      },
      {
        type: "success",
        trackId: "invoice-123",
        documentName: "Invoice.pdf",
        timestamp: new Date(),
      },
    ];

    // Filter to success only, then search for "report"
    const filtered = filterByStatus(items, "success");
    const result = searchItems(filtered, "report");

    expect(result).toHaveLength(1);
    expect(result[0].trackId).toBe("report-abc");
  });

  it("full pipeline: filter -> search -> sort -> limit", () => {
    const now = new Date();
    const items: HistoryItem[] = [
      {
        type: "success",
        trackId: "a-report",
        timestamp: new Date(now.getTime() - 1000),
      },
      { type: "success", trackId: "b-report", timestamp: now },
      {
        type: "success",
        trackId: "c-report",
        timestamp: new Date(now.getTime() - 2000),
      },
      { type: "failed", trackId: "d-report", timestamp: now },
    ];

    let result = filterByStatus(items, "success");
    result = searchItems(result, "report");
    result = sortAndLimit(result, 2);

    expect(result).toHaveLength(2);
    expect(result[0].trackId).toBe("b-report"); // Most recent
    expect(result[1].trackId).toBe("a-report"); // Second most recent
  });
});
