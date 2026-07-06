/**
 * Model picker option list utilities (SPEC-043 DRY / SRP).
 *
 * Single place for deduplication, selection pinning, and provider grouping.
 */
import { getProviderDisplayName } from "@/lib/provider-display";
import type { ModelPickerOption, ModelPickerValue } from "./model-picker-panel";

/** Stable key for provider/model identity (case-insensitive). */
export function pickerOptionKey(fullId: string): string {
  return fullId.toLowerCase();
}

/** Prefer live / richer metadata when merging duplicate fullIds. */
function optionRichness(option: ModelPickerOption): number {
  let score = 0;
  if (option.isLive) score += 4;
  if (option.contextLength > 0) score += 2;
  if (option.supportsVision) score += 1;
  if (option.supportsTools) score += 1;
  if (option.available === true) score += 1;
  if (option.deprecated) score -= 8;
  if (option.available === false) score -= 4;
  return score;
}

/** Merge lists, keeping one entry per fullId (richest wins). */
export function mergePickerOptions(
  ...lists: ReadonlyArray<ReadonlyArray<ModelPickerOption>>
): ModelPickerOption[] {
  const byKey = new Map<string, ModelPickerOption>();
  for (const list of lists) {
    for (const option of list) {
      const key = pickerOptionKey(option.fullId);
      const existing = byKey.get(key);
      if (!existing || optionRichness(option) > optionRichness(existing)) {
        byKey.set(key, option);
      }
    }
  }
  return Array.from(byKey.values());
}

/** Ensure the current value appears once in the list (after dedupe). */
export function ensureSelectedInPickerOptions(
  list: ModelPickerOption[],
  value: ModelPickerValue | undefined,
  catalog: ReadonlyArray<ModelPickerOption>,
): ModelPickerOption[] {
  const deduped = mergePickerOptions(list);
  if (!value) return deduped;

  const key = pickerOptionKey(value.fullId);
  if (deduped.some((o) => pickerOptionKey(o.fullId) === key)) {
    return deduped;
  }

  const fromCatalog = catalog.find((o) => pickerOptionKey(o.fullId) === key);
  if (fromCatalog) {
    return mergePickerOptions([fromCatalog], deduped);
  }

  return mergePickerOptions(
    [
      {
        provider: value.provider,
        providerDisplayName: getProviderDisplayName(value.provider),
        name: value.model,
        displayName: value.model,
        fullId: value.fullId,
        contextLength: 0,
        supportsVision: false,
        supportsTools: false,
        supportsStreaming: true,
      },
    ],
    deduped,
  );
}

/** Group by provider; optional provider shown first. Groups are deduped per fullId. */
export function groupPickerOptionsByProvider(
  options: ReadonlyArray<ModelPickerOption>,
  priorityProvider?: string | null,
): Map<string, ModelPickerOption[]> {
  const map = new Map<string, ModelPickerOption[]>();
  for (const option of mergePickerOptions(options)) {
    const group = map.get(option.provider) ?? [];
    group.push(option);
    map.set(option.provider, group);
  }

  if (!priorityProvider) return map;

  const matchKey = Array.from(map.keys()).find(
    (id) => id.toLowerCase() === priorityProvider.toLowerCase(),
  );
  if (!matchKey) return map;

  const ordered = new Map<string, ModelPickerOption[]>();
  ordered.set(matchKey, map.get(matchKey)!);
  for (const [id, models] of map) {
    if (id !== matchKey) ordered.set(id, models);
  }
  return ordered;
}
