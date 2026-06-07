import { describe, expect, it } from "vitest";
import {
  buildDocumentNameMap,
  countDocumentsByPhase,
  formatDurationSeconds,
  formatPipelineCost,
  formatTaskType,
  formatThroughput,
  formatTokenCount,
  formatWaitTimeMs,
  partitionTasksByStatus,
  replaceUuidsInMessage,
} from "../pipeline-formatters";
import type { TaskResponse } from "@/types";

describe("pipeline-formatters (UI-P3-004)", () => {
  it("counts documents by pipeline phase", () => {
    expect(
      countDocumentsByPhase([
        "pending",
        "processing",
        "chunking",
        "completed",
        "indexed",
        "failed",
        "cancelled",
        "unknown",
      ]),
    ).toEqual({
      pending: 2,
      processing: 2,
      completed: 2,
      failed: 2,
    });
  });

  it("returns zero counts for empty input", () => {
    expect(countDocumentsByPhase([])).toEqual({
      pending: 0,
      processing: 0,
      completed: 0,
      failed: 0,
    });
  });

  it("formats task types for display", () => {
    expect(formatTaskType("text_insert")).toBe("Text Insert");
  });

  it("formats cost, duration, tokens, and throughput", () => {
    expect(formatPipelineCost(0)).toBe("< $0.0001");
    expect(formatPipelineCost(0.005)).toBe("$0.0050");
    expect(formatDurationSeconds(45)).toBe("45s");
    expect(formatDurationSeconds(125)).toBe("2m 5s");
    expect(formatTokenCount(500)).toBe("500");
    expect(formatTokenCount(1500)).toBe("1.5K");
    expect(formatThroughput(0.05)).toBe("< 0.1/min");
    expect(formatThroughput(2.4)).toBe("2/min");
  });

  it("formats wait time from milliseconds", () => {
    expect(formatWaitTimeMs(30_000)).toBe("30s");
    expect(formatWaitTimeMs(90_000)).toBe("1m 30s");
    expect(formatWaitTimeMs(3_720_000)).toBe("1h 2m");
  });

  it("replaces UUIDs in messages with document names", () => {
    const id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
    const map = new Map([[id, "Annual Report 2024"]]);
    expect(
      replaceUuidsInMessage(`Processing document ${id}`, map),
    ).toBe("Processing document Annual Report 2024");
    expect(
      replaceUuidsInMessage(`Processing document ${id}`, new Map()),
    ).toBe("Processing document doc-a1b2c3d4");
  });

  it("builds document name map with fallbacks", () => {
    const map = buildDocumentNameMap([
      { id: "ABC", title: "Title A" },
      { id: "DEF", file_name: "file.pdf" },
      { id: "12345678-1234-1234-1234-123456789012" },
    ]);
    expect(map.get("abc")).toBe("Title A");
    expect(map.get("def")).toBe("file.pdf");
    expect(map.get("12345678-1234-1234-1234-123456789012")).toBe(
      "Document 12345678",
    );
  });

  it("partitions and sorts tasks by status", () => {
    const tasks = [
      {
        track_id: "2",
        status: "pending",
        created_at: "2026-01-02T00:00:00Z",
        task_type: "text_insert",
      },
      {
        track_id: "1",
        status: "pending",
        created_at: "2026-01-01T00:00:00Z",
        task_type: "text_insert",
      },
      {
        track_id: "3",
        status: "processing",
        created_at: "2026-01-03T00:00:00Z",
        task_type: "text_insert",
      },
    ] as TaskResponse[];

    const { pendingTasks, processingTasks } = partitionTasksByStatus(tasks);
    expect(pendingTasks.map((t) => t.track_id)).toEqual(["1", "2"]);
    expect(processingTasks.map((t) => t.track_id)).toEqual(["3"]);
  });
});
