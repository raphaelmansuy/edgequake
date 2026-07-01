import { describe, expect, test } from "bun:test";
import {
  ADMIT_PROGRESS_PERCENT,
  formatUploadMegabytes,
  transferProgressPercent,
  uploadTimeoutMs,
} from "@/lib/upload/upload-timeout";

describe("uploadTimeoutMs", () => {
  test("returns base for empty size", () => {
    expect(uploadTimeoutMs(0)).toBe(60_000);
  });

  test("scales with file size", () => {
    expect(uploadTimeoutMs(1024 * 1024)).toBe(68_000);
    expect(uploadTimeoutMs(11 * 1024 * 1024)).toBe(148_000);
  });

  test("caps at maximum", () => {
    expect(uploadTimeoutMs(200 * 1024 * 1024)).toBe(600_000);
  });
});

describe("transferProgressPercent", () => {
  test("maps bytes to 5-85 band", () => {
    expect(transferProgressPercent(0, 1000)).toBe(5);
    expect(transferProgressPercent(500, 1000)).toBe(45);
    expect(transferProgressPercent(1000, 1000)).toBe(85);
  });
});

describe("formatUploadMegabytes", () => {
  test("formats one decimal MB", () => {
    expect(formatUploadMegabytes(11_043_120)).toBe("10.5");
  });
});

describe("ADMIT_PROGRESS_PERCENT", () => {
  test("is in admit band", () => {
    expect(ADMIT_PROGRESS_PERCENT).toBeGreaterThan(85);
    expect(ADMIT_PROGRESS_PERCENT).toBeLessThan(95);
  });
});
