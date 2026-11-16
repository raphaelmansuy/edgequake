import { describe, expect, it } from "vitest";

import { groupByProvider, isMovingAlias, modelMatchesQuery } from "./model-jobs";

describe("model jobs", () => {
  it("treats an empty search as every model", () => {
    expect(modelMatchesQuery("Gemma 3", "")).toBe(true);
    expect(modelMatchesQuery("Gemma 3", "   ")).toBe(true);
  });

  it("matches a model or its provider, ignoring case", () => {
    expect(modelMatchesQuery("LM Studio Gemma", "gemma")).toBe(true);
    expect(modelMatchesQuery("LM Studio Gemma", "mistral")).toBe(false);
  });

  it("recognises a name that follows the loaded model", () => {
    expect(isMovingAlias("Currently Loaded Model")).toBe(true);
    expect(isMovingAlias("LM Studio · loaded model")).toBe(true);
    expect(isMovingAlias("Gemma 3n E4B")).toBe(false);
  });

  it("groups by provider in first-seen order", () => {
    const rows = [
      { provider: "LM Studio", name: "Gemma" },
      { provider: "Mistral", name: "Devstral" },
      { provider: "LM Studio", name: "Granite" },
    ];
    expect(groupByProvider(rows, (row) => row.provider).map((group) => [group.provider, group.rows.length])).toEqual([
      ["LM Studio", 2],
      ["Mistral", 1],
    ]);
  });
});
