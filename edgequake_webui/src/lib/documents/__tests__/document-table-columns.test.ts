import { describe, expect, test } from "bun:test";
import {
  DOCUMENT_TABLE_COL_PERCENTS,
  documentTableColPercentSum,
} from "../document-table-columns";

describe("document table column widths", () => {
  test("default layout percents sum to 100 and reserve Title", () => {
    expect(documentTableColPercentSum(false)).toBe(100);
    expect(Number.parseFloat(DOCUMENT_TABLE_COL_PERCENTS.default.title)).toBeGreaterThanOrEqual(24);
    expect(Number.parseFloat(DOCUMENT_TABLE_COL_PERCENTS.default.status)).toBeGreaterThanOrEqual(14);
  });

  test("cost layout percents sum to 100 and keep Title above 20%", () => {
    expect(documentTableColPercentSum(true)).toBe(100);
    expect(Number.parseFloat(DOCUMENT_TABLE_COL_PERCENTS.withCost.title)).toBeGreaterThanOrEqual(20);
  });

  test("ABAC layouts sum to 100", () => {
    expect(documentTableColPercentSum(false, true)).toBe(100);
    expect(documentTableColPercentSum(true, true)).toBe(100);
    expect(Number.parseFloat(DOCUMENT_TABLE_COL_PERCENTS.withAbac.class)).toBeGreaterThan(0);
  });
});
