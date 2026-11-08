import { describe, expect, it } from "vitest";

import type { CatalogEntry, EligibleModel, PolicyRow } from "@/lib/api/edgequake/model-catalog";

import {
  applyPick,
  brokenMessage,
  buildLlmRoles,
  ceilingDisplayRows,
  ceilingRowsFromSelection,
  disabledFromRows,
  draftFromWorkspaceRoles,
  isDraftDirty,
  narrowingMatrix,
  narrowingOptions,
  narrowingRowsFromDisabled,
  pairKey,
  samePolicyRows,
  pickerGroups,
  pickerValue,
  selectionFromCeilingRows,
  togglePair,
  WRITABLE_ROLE_USAGES,
} from "./model-usage";

const eligible = (provider: string, model: string, display = model, vision = false): EligibleModel => ({
  provider,
  provider_display_name: provider === "ollama" ? "Ollama" : "OpenAI",
  model,
  display_name: display,
  supports_vision: vision,
});

describe("pairKey", () => {
  it("normalises provider case and trims both parts", () => {
    expect(pairKey(" OpenAI ", " gpt-5-nano ")).toBe("openai::gpt-5-nano");
    expect(pairKey("openai", "gpt-5-nano")).toBe(pairKey("OPENAI", "gpt-5-nano"));
  });
});

describe("workspace role drafts", () => {
  it("never offers decision as a writable role", () => {
    expect(WRITABLE_ROLE_USAGES).not.toContain("decision");
    expect(WRITABLE_ROLE_USAGES).toContain("extract");
  });

  it("builds a draft per writable usage, empty when nothing is saved", () => {
    const draft = draftFromWorkspaceRoles(undefined);
    expect(Object.keys(draft).sort()).toEqual([...WRITABLE_ROLE_USAGES].sort());
    expect(draft.query).toEqual({ provider: "", model: "", connection_id: "" });
  });

  it("omits empty rows from the llm_roles payload", () => {
    const draft = draftFromWorkspaceRoles({ query: { provider: "openai", model: "gpt-5-nano" } });
    expect(buildLlmRoles(draft)).toEqual({
      query: { provider: "openai", model: "gpt-5-nano", connection_id: undefined },
    });
  });

  it("keeps a connection-only row so the connection choice is not lost", () => {
    const draft = draftFromWorkspaceRoles({ vlm: { connection_id: "conn-1" } });
    expect(buildLlmRoles(draft)).toEqual({
      vlm: { provider: undefined, model: undefined, connection_id: "conn-1" },
    });
  });

  it("detects dirty drafts", () => {
    const saved = draftFromWorkspaceRoles({ extract: { provider: "ollama", model: "gemma3" } });
    expect(isDraftDirty(saved, saved)).toBe(false);
    const changed = { ...saved, extract: { ...saved.extract!, model: "qwen3" } };
    expect(isDraftDirty(changed, saved)).toBe(true);
  });

  it("applyPick sets provider and model and keeps the connection", () => {
    const row = { provider: "", model: "", connection_id: "conn-1" };
    const picked = applyPick(row, { value: "", provider: "ollama", model: "gemma3", label: "", supportsVision: false });
    expect(picked).toEqual({ provider: "ollama", model: "gemma3", connection_id: "conn-1" });
    expect(applyPick(picked, undefined)).toEqual({ provider: "", model: "", connection_id: "conn-1" });
  });
});

describe("pickers", () => {
  const models = [
    eligible("openai", "gpt-5-nano", "GPT-5 nano"),
    eligible("ollama", "gemma3", "Gemma 3"),
    eligible("ollama", "qwen3", "Qwen 3"),
  ];

  it("groups by provider with sorted options", () => {
    const groups = pickerGroups(models);
    expect(groups.map((g) => g.label)).toEqual(["Ollama", "OpenAI"]);
    expect(groups[0].options.map((o) => o.label)).toEqual(["Gemma 3", "Qwen 3"]);
  });

  it("shows the saved pair even when it is no longer eligible, and blank when inheriting", () => {
    expect(pickerValue({ provider: "ollama", model: "gemma3", connection_id: "" })).toBe(pairKey("ollama", "gemma3"));
    expect(pickerValue({ provider: "ollama", model: "gone", connection_id: "" })).toBe(pairKey("ollama", "gone"));
    expect(pickerValue({ provider: "", model: "", connection_id: "" })).toBe("");
  });
});

describe("tenant ceiling", () => {
  const entries: CatalogEntry[] = [
    { provider: "openai", provider_display_name: "OpenAI", model: "gpt-5-nano", display_name: "GPT-5 nano", health: "healthy", usages: ["query", "extract"] },
    { provider: "ollama", provider_display_name: "Ollama", model: "gemma3", display_name: "Gemma 3", health: "unknown", usages: ["query"] },
  ];

  it("toggles a pair without mutating the input", () => {
    const start = {};
    const on = togglePair(start, "query", pairKey("openai", "gpt-5-nano"));
    expect(start).toEqual({});
    expect(on.query).toEqual([pairKey("openai", "gpt-5-nano")]);
    const off = togglePair(on, "query", pairKey("openai", "gpt-5-nano"));
    expect(off.query).toEqual([]);
  });

  it("emits rows in usage order and skips empty usages", () => {
    let selection = togglePair({}, "query", pairKey("ollama", "gemma3"));
    selection = togglePair(selection, "extract", pairKey("openai", "gpt-5-nano"));
    expect(ceilingRowsFromSelection(selection)).toEqual([
      { provider: "openai", model: "gpt-5-nano", usage: "extract", enabled: true },
      { provider: "ollama", model: "gemma3", usage: "query", enabled: true },
    ]);
  });

  it("round-trips rows through the selection", () => {
    const rows: PolicyRow[] = [
      { provider: "ollama", model: "gemma3", usage: "query", enabled: true },
      { provider: "ollama", model: "gemma3", usage: "vlm", enabled: true },
    ];
    const selection = selectionFromCeilingRows(rows);
    expect(ceilingRowsFromSelection(selection)).toEqual(rows);
  });

  it("shows every registered model, and keeps a saved pair that left the catalog", () => {
    const selection = { query: [pairKey("retired", "old-model")], extract: [pairKey("openai", "gpt-5-nano")] };
    const rows = ceilingDisplayRows(entries, selection);
    expect(rows.map((r) => [r.registered, r.key])).toEqual([
      [true, pairKey("openai", "gpt-5-nano")],
      [true, pairKey("ollama", "gemma3")],
      [false, pairKey("retired", "old-model")],
    ]);
    expect(rows[2].usages).toEqual(["query"]);
    expect(rows[0].usages).toEqual(["query", "extract"]);
    expect(rows[0].providerLabel).toBe("OpenAI");
    expect(rows[0].modelLabel).toBe("GPT-5 nano");
    expect(rows[2].providerLabel).toBe("retired");
    expect(rows[2].modelLabel).toBe("old-model");
  });
});

describe("workspace narrowing", () => {
  it("round-trips disabled pairs", () => {
    const rows: PolicyRow[] = [{ provider: "ollama", model: "gemma3", usage: "query", enabled: false }];
    const disabled = disabledFromRows(rows);
    expect(disabled.query).toEqual([pairKey("ollama", "gemma3")]);
    expect(narrowingRowsFromDisabled(disabled)).toEqual(rows);
  });

  it("ignores enabled rows when reading disabled pairs", () => {
    expect(disabledFromRows([{ provider: "a", model: "b", usage: "query", enabled: true }])).toEqual({});
  });

  it("offers eligible models plus disabled pairs that fell out of the eligible list", () => {
    const options = narrowingOptions(
      [eligible("ollama", "gemma3", "Gemma 3")],
      [pairKey("openai", "gpt-4.1-nano")],
    );
    expect(options.map((o) => o.label).sort()).toEqual(["Gemma 3", "gpt-4.1-nano"]);
    const retired = options.find((o) => o.value === pairKey("openai", "gpt-4.1-nano"));
    expect(retired).toMatchObject({ provider: "openai", model: "gpt-4.1-nano" });
  });

  it("does not duplicate a disabled pair that is also eligible", () => {
    const options = narrowingOptions(
      [eligible("ollama", "gemma3", "Gemma 3")],
      [pairKey("ollama", "gemma3")],
    );
    expect(options).toHaveLength(1);
  });
});

describe("brokenMessage", () => {
  it("returns null for a healthy pair and a sentence for each broken reason", () => {
    expect(brokenMessage("none")).toBeNull();
    for (const reason of ["not_eligible", "capability", "credential", "probe", "cross_tenant"] as const) {
      expect(brokenMessage(reason)).toEqual(expect.any(String));
    }
  });
});

describe("narrowingMatrix", () => {
  it("merges usages per model and marks what is turned off", () => {
    const rows = narrowingMatrix(
      {
        query: [eligible("ollama", "gemma3", "Gemma 3")],
        extract: [eligible("ollama", "gemma3", "Gemma 3"), eligible("openai", "gpt-5-nano", "GPT-5 nano")],
      },
      { query: [pairKey("ollama", "gemma3")] },
    );
    expect(rows.map((r) => r.value)).toEqual([pairKey("ollama", "gemma3"), pairKey("openai", "gpt-5-nano")]);
    expect([...rows[0].presentIn].sort()).toEqual(["extract", "query"]);
    expect([...rows[1].presentIn]).toEqual(["extract"]);
  });

  it("keeps a disabled model visible in the usage it was turned off for", () => {
    const rows = narrowingMatrix({}, { vlm: [pairKey("openai", "old-model")] });
    expect(rows).toHaveLength(1);
    expect([...rows[0].presentIn]).toEqual(["vlm"]);
  });
});

describe("samePolicyRows", () => {
  it("ignores row order", () => {
    const a: PolicyRow[] = [
      { provider: "a", model: "1", usage: "query", enabled: true },
      { provider: "b", model: "2", usage: "extract", enabled: true },
    ];
    expect(samePolicyRows(a, [...a].reverse())).toBe(true);
  });

  it("treats enabled and disabled rows as different", () => {
    const on: PolicyRow[] = [{ provider: "a", model: "1", usage: "query", enabled: true }];
    const off: PolicyRow[] = [{ provider: "a", model: "1", usage: "query", enabled: false }];
    expect(samePolicyRows(on, off)).toBe(false);
  });
});
