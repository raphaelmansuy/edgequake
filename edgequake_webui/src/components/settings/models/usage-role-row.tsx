"use client";

import { useEffect, useMemo } from "react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { ConnectionOption } from "@/hooks/use-model-policy";
import { selectableConnections } from "@/lib/settings/connection-options";
import { useEligibleModels } from "@/hooks/use-model-policy";
import type { ResolvedUsage, UsageKey } from "@/lib/api/edgequake/model-catalog";
import {
  applyPick,
  brokenMessage,
  pairKey,
  pickerGroups,
  pickerValue,
  type RoleDraft,
  SOURCE_LABELS,
  USAGE_LABELS,
} from "@/lib/settings/model-usage";

interface UsageRoleRowProps {
  usage: UsageKey;
  workspaceId: string;
  /** Undefined for a usage with no role to write (`decision`): the row only reports. */
  draft?: RoleDraft;
  resolved: ResolvedUsage | undefined;
  connections: readonly ConnectionOption[];
  readOnly: boolean;
  onChange: (next: RoleDraft) => void;
  /**
   * SPEC-184 — reports whether this row is currently flagging a saved model that cannot be used.
   *
   * WHY a callback instead of counting in the parent: the row is the only place that knows both
   * conditions (the server's resolved reason AND whether the saved pair is absent from the eligible
   * list). The header used to count only the first, so it said "3" while four rows were red.
   * Letting the rows report makes the summary and the table one fact by construction.
   */
  onFlagChange?: (usage: UsageKey, flagged: boolean) => void;
}

const selectClass =
  "w-full rounded-md border bg-background px-2 py-1.5 text-sm disabled:cursor-not-allowed disabled:opacity-70";

/**
 * The one row grid, shared by the header and every row so the columns line up.
 * Below `md` it stacks; the table only becomes a table when there is room for it.
 */
export const ROW_GRID =
  "grid gap-3 py-3 md:grid-cols-[minmax(0,1.1fr)_minmax(0,1.4fr)_minmax(0,1fr)] md:items-start";

/** The resolved pair for a usage, in words: "Ollama · gemma3 — Workspace role". */
function ResolvedSummary({ resolved }: { resolved: ResolvedUsage | undefined }) {
  if (!resolved) return <p className="text-xs text-muted-foreground">Not resolved yet.</p>;
  return (
    <div className="space-y-1 text-xs">
      <p className="text-muted-foreground" data-testid="usage-resolved">
        Using {resolved.provider_display_name ?? resolved.provider} · {resolved.model} —{" "}
        {SOURCE_LABELS[resolved.source]}
      </p>
      {resolved.locked ? <Badge variant="outline">Set by the operator</Badge> : null}
    </div>
  );
}

/**
 * Why the saved pair cannot be used, placed directly under the picker that fixes it.
 *
 * One short line, not two. The instruction ("pick another one to save") is identical on every
 * affected row, so repeating it four times made a tall, noisy table: the card states it once
 * above the rows. What stays here is the per-row fact — which model, and why it is refused.
 */
function BrokenPickNotice({
  resolved,
  savedButIneligible,
  inheriting,
}: {
  resolved: ResolvedUsage | undefined;
  savedButIneligible: boolean;
  /** True when the refused value comes from inheritance, not from a saved pick. */
  inheriting: boolean;
}) {
  /*
   * SPEC-184 — ONE phrase for one fact. The resolved reason and the picker's own check both mean
   * "this saved pair cannot run here", and they used to word it two different ways in the same
   * table ("Not allowed…" vs "This model is not allowed…"), which reads like two problems.
   *
   * An inherited value is refused too, but the reader is looking at a picker that says "Inherit",
   * so naming the source is the difference between an unexplained error and an obvious one.
   */
  const broken = brokenMessage(resolved?.broken_reason ?? "none");
  const reason =
    broken ??
    (savedButIneligible ? brokenMessage("not_eligible") : null);
  if (!reason) return null;
  return (
    <p role="alert" className="text-xs text-destructive" data-testid="usage-broken">
      {inheriting && broken ? `The inherited model is refused: ${reason}` : reason}
    </p>
  );
}

/** One usage: resolved pair, eligible-only model picker, and optional connection. */
export function UsageRoleRow({
  usage,
  workspaceId,
  draft,
  resolved,
  connections,
  readOnly,
  onChange,
  onFlagChange,
}: UsageRoleRowProps) {
  const label = USAGE_LABELS[usage];
  const eligible = useEligibleModels(draft ? workspaceId : null, usage);
  const groups = useMemo(() => pickerGroups(eligible.data?.models ?? []), [eligible.data]);
  const options = useMemo(() => groups.flatMap((g) => g.options), [groups]);

  const value = draft ? pickerValue(draft) : "";
  // Only judge eligibility once the list has loaded; a pending list would flash a false warning.
  const savedButIneligible =
    !eligible.isPending && !!value && !options.some((o) => o.value === value);
  /*
   * Report to the card so its summary counts exactly the rows that render a reason. A row without a
   * `draft` (the `decision` usage) has no picker and shows no reason, so it must not be counted —
   * otherwise the summary says 5 while four rows are red. Every hook runs before the early return.
   */
  const flagged = draft
    ? (resolved?.broken_reason ?? "none") !== "none" || savedButIneligible
    : false;
  useEffect(() => {
    onFlagChange?.(usage, flagged);
  }, [flagged, onFlagChange, usage]);

  if (!draft) {
    return (
      <div data-testid={`usage-row-${usage}`} className={ROW_GRID}>
        <div className="space-y-1">
          <p className="text-sm font-medium">{label}</p>
          <p className="text-xs text-muted-foreground">Follows the workspace default (it has no model of its own).</p>
        </div>
        <div className="md:col-span-2">
          <ResolvedSummary resolved={resolved} />
        </div>
      </div>
    );
  }

  const locked = resolved?.locked ?? false;
  const controlsDisabled = readOnly || locked;

  return (
    <div
      data-testid={`usage-row-${usage}`}
      className={ROW_GRID}
    >
      <div className="space-y-1">
        <p className="text-sm font-medium">{label}</p>
        <ResolvedSummary resolved={resolved} />
      </div>
      <div className="space-y-1">
        <label className="text-xs text-muted-foreground md:sr-only" htmlFor={`usage-model-${usage}`}>
          Model
        </label>
        <select
          id={`usage-model-${usage}`}
          data-testid={`usage-model-${usage}`}
          aria-label={`Model for ${label}`}
          className={cn(selectClass, savedButIneligible && "border-destructive")}
          value={value}
          disabled={controlsDisabled || eligible.isPending}
          onChange={(e) => onChange(applyPick(draft, options.find((o) => o.value === e.target.value)))}
        >
          <option value="">{eligible.isPending ? "Loading models…" : "Inherit"}</option>
          {savedButIneligible ? (
            <option value={pairKey(draft.provider, draft.model)} disabled>
              {draft.model} (not eligible)
            </option>
          ) : null}
          {groups.map((group) => (
            <optgroup key={group.provider} label={group.label}>
              {group.options.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                  {o.supportsVision ? " · vision" : ""}
                </option>
              ))}
            </optgroup>
          ))}
        </select>
        <BrokenPickNotice resolved={resolved} savedButIneligible={savedButIneligible} inheriting={!value} />
      </div>
      <div className="space-y-1">
        <label className="text-xs text-muted-foreground md:sr-only" htmlFor={`usage-connection-${usage}`}>
          Connection
        </label>
        <select
          id={`usage-connection-${usage}`}
          data-testid={`usage-connection-${usage}`}
          aria-label={`Connection for ${label}`}
          className={selectClass}
          value={draft.connection_id}
          disabled={controlsDisabled}
          onChange={(e) => onChange({ ...draft, connection_id: e.target.value })}
        >
          <option value="">Inherit</option>
          {selectableConnections(connections).map((c) => (
            <option key={`${c.source}-${c.slug}-${c.id}`} value={c.id}>
              {c.display_name}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
