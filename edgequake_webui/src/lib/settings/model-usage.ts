/**
 * SPEC-175 W4 — pure rules for the typed model editors. No React, no I/O.
 *
 * - The workspace role editor writes `llm_roles` (`WRITABLE_ROLE_USAGES`). `decision` has no
 *   role; it follows the workspace default and is shown read-only.
 * - The tenant ceiling and the workspace narrowing are sets of `(provider, model)` pairs per
 *   usage. A usage with no rows is unconstrained, so "clear" always means "any model".
 * - Pairs are compared on a single key, `pairKey`, so picker values and policy rows agree.
 */

import {
  USAGE_KEYS,
  type BrokenReason,
  type CatalogEntry,
  type EligibleModel,
  type PolicyRow,
  type ResolutionSource,
  type UsageKey,
} from "@/lib/api/edgequake/model-catalog";

/** Usages that map to a workspace `llm_roles` key. `decision` has none. */
export const WRITABLE_ROLE_USAGES: readonly UsageKey[] = USAGE_KEYS.filter((u) => u !== "decision");

export const pairKey = (provider: string, model: string): string =>
  `${provider.trim().toLowerCase()}::${model.trim()}`;

export const USAGE_LABELS: Record<UsageKey, string> = {
  extract: "Extraction",
  query: "Query",
  summary: "Summary",
  vlm: "Vision (PDF pages)",
  keyword: "Keywords",
  embedding: "Embedding",
  decision: "Decision",
};

// ---------------------------------------------------------------------------
// Workspace role drafts (writes `llm_roles`)
// ---------------------------------------------------------------------------

export interface RoleDraft {
  provider: string;
  model: string;
  connection_id: string;
}

export type RoleDraftMap = Partial<Record<UsageKey, RoleDraft>>;

export interface LlmRoleValue {
  provider?: string;
  model?: string;
  connection_id?: string;
}

type LlmRolesInput = Record<string, { provider?: string; model?: string; connection_id?: string } | undefined>;

const EMPTY_DRAFT: RoleDraft = { provider: "", model: "", connection_id: "" };

/** The saved workspace roles, one draft per writable usage. */
export function draftFromWorkspaceRoles(roles: LlmRolesInput | undefined): RoleDraftMap {
  const draft: RoleDraftMap = {};
  for (const usage of WRITABLE_ROLE_USAGES) {
    const saved = roles?.[usage];
    draft[usage] = {
      provider: saved?.provider ?? "",
      model: saved?.model ?? "",
      connection_id: saved?.connection_id ?? "",
    };
  }
  return draft;
}

/** Rows with nothing set are left out, so an empty role inherits the workspace default. */
export function buildLlmRoles(draft: RoleDraftMap): Record<string, LlmRoleValue> {
  const roles: Record<string, LlmRoleValue> = {};
  for (const usage of WRITABLE_ROLE_USAGES) {
    const row = draft[usage] ?? EMPTY_DRAFT;
    if (!row.provider && !row.model && !row.connection_id) continue;
    roles[usage] = {
      provider: row.provider || undefined,
      model: row.model || undefined,
      connection_id: row.connection_id || undefined,
    };
  }
  return roles;
}

/** Whether the draft differs from what the workspace last saved. */
export function isDraftDirty(draft: RoleDraftMap, saved: RoleDraftMap): boolean {
  return WRITABLE_ROLE_USAGES.some((usage) => {
    const a = draft[usage] ?? EMPTY_DRAFT;
    const b = saved[usage] ?? EMPTY_DRAFT;
    return a.provider !== b.provider || a.model !== b.model || a.connection_id !== b.connection_id;
  });
}

// ---------------------------------------------------------------------------
// Pickers (eligible models only)
// ---------------------------------------------------------------------------

export interface PickerOption {
  value: string;
  provider: string;
  model: string;
  /** Provider name as a person reads it. Falls back to the slug. */
  providerLabel?: string;
  label: string;
  supportsVision: boolean;
}

export interface PickerGroup {
  provider: string;
  label: string;
  options: PickerOption[];
}

/** Group eligible models by provider. Providers and models are sorted for a stable list. */
export function pickerGroups(models: readonly EligibleModel[]): PickerGroup[] {
  const groups = new Map<string, PickerGroup>();
  for (const m of models) {
    const key = m.provider.toLowerCase();
    const group =
      groups.get(key) ??
      { provider: m.provider, label: m.provider_display_name || m.provider, options: [] };
    group.options.push({
      value: pairKey(m.provider, m.model),
      provider: m.provider,
      model: m.model,
      providerLabel: m.provider_display_name || m.provider,
      label: m.display_name || m.model,
      supportsVision: m.supports_vision,
    });
    groups.set(key, group);
  }
  const sorted = [...groups.values()].sort((a, b) => a.label.localeCompare(b.label));
  for (const group of sorted) {
    group.options.sort((a, b) => a.label.localeCompare(b.label));
  }
  return sorted;
}

/**
 * The value the picker shows for a draft: the saved pair, even when it is no longer eligible, so
 * the user sees what is saved. "" when the row inherits the default.
 */
export function pickerValue(row: RoleDraft): string {
  return row.provider && row.model ? pairKey(row.provider, row.model) : "";
}

/** A draft row after the user picks an option. The connection is kept. */
export function applyPick(row: RoleDraft, option: PickerOption | undefined): RoleDraft {
  if (!option) return { ...row, provider: "", model: "" };
  return { ...row, provider: option.provider, model: option.model };
}

// ---------------------------------------------------------------------------
// Tenant ceiling (tenant managers)
// ---------------------------------------------------------------------------

/** For each usage, the pair keys the tenant allows. Absent or empty means unconstrained. */
export type PairSelection = Partial<Record<UsageKey, string[]>>;

const sortPairs = (pairs: readonly string[]) => [...new Set(pairs)].sort();

export function selectionFromCeilingRows(rows: readonly PolicyRow[]): PairSelection {
  const selection: PairSelection = {};
  for (const row of rows) {
    if (!row.enabled) continue;
    selection[row.usage] = sortPairs([...(selection[row.usage] ?? []), pairKey(row.provider, row.model)]);
  }
  return selection;
}

/** Ceiling rows for the non-empty usages, in `USAGE_KEYS` order. */
export function ceilingRowsFromSelection(selection: PairSelection): PolicyRow[] {
  const rows: PolicyRow[] = [];
  for (const usage of USAGE_KEYS) {
    for (const key of sortPairs(selection[usage] ?? [])) {
      const [provider, model] = splitPairKey(key);
      rows.push({ provider, model, usage, enabled: true });
    }
  }
  return rows;
}

/** Toggle one pair for one usage. Returns a new selection, never mutates the input. */
export function togglePair(selection: PairSelection, usage: UsageKey, key: string): PairSelection {
  const current = selection[usage] ?? [];
  const next = current.includes(key) ? current.filter((k) => k !== key) : [...current, key];
  return { ...selection, [usage]: sortPairs(next) };
}

/** One ceiling row. A saved pair that is no longer registered is kept so a save never drops it. */
export interface CeilingDisplayRow {
  key: string;
  /** `${provider} · ${model}`, the name checkboxes announce. */
  label: string;
  providerLabel: string;
  modelLabel: string;
  health: CatalogEntry["health"] | null;
  /** Usages the row can be ticked for: a registered model's usages, or the saved ones for an orphan. */
  usages: readonly UsageKey[];
  registered: boolean;
}

export function ceilingDisplayRows(entries: readonly CatalogEntry[], selection: PairSelection): CeilingDisplayRow[] {
  const registered = new Set<string>();
  const rows: CeilingDisplayRow[] = entries.map((e) => {
    const key = pairKey(e.provider, e.model);
    registered.add(key);
    return {
      key,
      label: `${e.provider_display_name} · ${e.display_name}`,
      providerLabel: e.provider_display_name,
      modelLabel: e.display_name,
      health: e.health,
      usages: e.usages,
      registered: true,
    };
  });
  const orphans = new Set<string>();
  for (const usage of USAGE_KEYS) {
    for (const key of selection[usage] ?? []) if (!registered.has(key)) orphans.add(key);
  }
  for (const key of [...orphans].sort()) {
    const [provider, model] = splitPairKey(key);
    rows.push({
      key,
      label: `${provider} · ${model}`,
      providerLabel: provider,
      modelLabel: model,
      health: null,
      usages: USAGE_KEYS.filter((u) => selection[u]?.includes(key)),
      registered: false,
    });
  }
  return rows;
}

// ---------------------------------------------------------------------------
// Workspace narrowing (workspace admins)
// ---------------------------------------------------------------------------

/** For each usage, the pair keys this workspace has turned off. */
export type DisabledSelection = PairSelection;

export function disabledFromRows(rows: readonly PolicyRow[]): DisabledSelection {
  const disabled: DisabledSelection = {};
  for (const row of rows) {
    if (row.enabled) continue;
    disabled[row.usage] = sortPairs([...(disabled[row.usage] ?? []), pairKey(row.provider, row.model)]);
  }
  return disabled;
}

/** Narrowing rows: one `enabled: false` row per disabled pair. */
export function narrowingRowsFromDisabled(disabled: DisabledSelection): PolicyRow[] {
  return ceilingRowsFromSelection(disabled).map((row) => ({ ...row, enabled: false }));
}

/** One model across every usage it appears in, for the narrowing matrix. */
export interface NarrowingRow {
  value: string;
  provider: string;
  providerLabel: string;
  model: string;
  label: string;
  presentIn: Set<UsageKey>;
}

/** The narrowing matrix: one row per model, the usages it can serve, and what is turned off. */
export function narrowingMatrix(
  eligibleByUsage: Partial<Record<UsageKey, readonly EligibleModel[]>>,
  disabled: DisabledSelection,
): NarrowingRow[] {
  const rows = new Map<string, NarrowingRow>();
  for (const usage of USAGE_KEYS) {
    const options = narrowingOptions(eligibleByUsage[usage] ?? [], disabled[usage] ?? []);
    for (const option of options) {
      const row = rows.get(option.value) ?? {
        value: option.value,
        provider: option.provider,
        providerLabel: option.providerLabel || option.provider,
        model: option.model,
        label: option.label,
        presentIn: new Set<UsageKey>(),
      };
      row.presentIn.add(usage);
      rows.set(option.value, row);
    }
  }
  return [...rows.values()].sort((a, b) => a.label.localeCompare(b.label) || a.value.localeCompare(b.value));
}

/** Whether two policy row lists describe the same ceiling or narrowing (order-insensitive). */
export function samePolicyRows(a: readonly PolicyRow[], b: readonly PolicyRow[]): boolean {
  const normalise = (rows: readonly PolicyRow[]) =>
    rows.map((r) => `${r.usage}|${pairKey(r.provider, r.model)}|${r.enabled}`).sort().join("\n");
  return normalise(a) === normalise(b);
}

/** Options for one usage: the eligible models plus anything this workspace has turned off. */
export function narrowingOptions(
  eligible: readonly EligibleModel[],
  disabledKeys: readonly string[],
): PickerOption[] {
  const seen = new Set<string>();
  const options: PickerOption[] = [];
  for (const m of eligible) {
    const value = pairKey(m.provider, m.model);
    seen.add(value);
    options.push({
      value,
      provider: m.provider,
      providerLabel: m.provider_display_name || m.provider,
      model: m.model,
      label: m.display_name || m.model,
      supportsVision: m.supports_vision,
    });
  }
  for (const key of disabledKeys) {
    if (seen.has(key)) continue;
    const [provider, model] = splitPairKey(key);
    options.push({ value: key, provider, model, label: model, supportsVision: false });
  }
  return options.sort((a, b) => a.label.localeCompare(b.label));
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

export const SOURCE_LABELS: Record<ResolutionSource, string> = {
  operator_lock: "Set by the operator",
  request: "Per request",
  workspace_role: "Workspace role",
  workspace_default: "Workspace default",
  tenant_default: "Organization default",
  server_config: "Server default",
  env_bootstrap: "Environment default",
  compiled_default: "Built-in default",
};

/** The reason a resolved pair cannot run, in words. `null` when it can. */
export function brokenMessage(reason: BrokenReason): string | null {
  switch (reason) {
    case "none":
      return null;
    case "not_eligible":
      return "This model is not allowed for this workspace.";
    case "capability":
      return "This model cannot do this job.";
    case "credential":
      return "No working key for this model's provider.";
    case "probe":
      return "The last connection check failed for this provider.";
    case "cross_tenant":
      return "This connection belongs to another organization.";
  }
}

function splitPairKey(key: string): [string, string] {
  const at = key.indexOf("::");
  return at < 0 ? [key, ""] : [key.slice(0, at), key.slice(at + 2)];
}
