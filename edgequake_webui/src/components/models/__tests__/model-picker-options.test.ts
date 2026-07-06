import { describe, expect, it } from "bun:test";
import type { ModelPickerOption } from "../model-picker-panel";
import {
  ensureSelectedInPickerOptions,
  mergePickerOptions,
  pickerOptionKey,
} from "../model-picker-options";

function option(
  provider: string,
  model: string,
  extras: Partial<ModelPickerOption> = {},
): ModelPickerOption {
  const fullId = `${provider}/${model}`;
  return {
    provider,
    providerDisplayName: provider,
    name: model,
    displayName: model,
    fullId,
    contextLength: 0,
    supportsVision: false,
    supportsTools: false,
    supportsStreaming: true,
    ...extras,
  };
}

describe("mergePickerOptions", () => {
  it("dedupes identical fullId from static and live catalogs", () => {
    const staticEntry = option("mistral", "mistral-large-2512", { contextLength: 128_000 });
    const liveEntry = option("mistral", "mistral-large-2512", { isLive: true, contextLength: 131_000 });

    const merged = mergePickerOptions([staticEntry, liveEntry], [liveEntry, staticEntry]);

    expect(merged).toHaveLength(1);
    expect(merged[0]?.isLive).toBe(true);
    expect(merged[0]?.contextLength).toBe(131_000);
  });

  it("uses case-insensitive fullId keys", () => {
    const merged = mergePickerOptions(
      [option("Mistral", "Model-A")],
      [option("mistral", "model-a")],
    );
    expect(merged).toHaveLength(1);
    expect(pickerOptionKey(merged[0]!.fullId)).toBe("mistral/model-a");
  });
});

describe("ensureSelectedInPickerOptions", () => {
  it("does not duplicate the selected value", () => {
    const list = [option("mistral", "mistral-large-2512")];
    const result = ensureSelectedInPickerOptions(list, {
      provider: "mistral",
      model: "mistral-large-2512",
      fullId: "mistral/mistral-large-2512",
    }, list);
    expect(result).toHaveLength(1);
  });
});
