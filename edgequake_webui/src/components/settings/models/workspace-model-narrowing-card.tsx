"use client";

/**
 * SPEC-175 W5 — workspace narrowing. Editors turn off models this workspace must not use, per
 * job. A tick means the workspace may still use the model. The organization ceiling still
 * applies: a model the ceiling forbids is not listed here.
 */

import { useMemo, useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { useEligibleModelsByUsage, useSaveWorkspaceModelPolicy, useWorkspaceModelPolicy } from "@/hooks/use-model-policy";
import { useServerBackedDraft } from "@/hooks/use-server-backed-draft";
import { USAGE_KEYS, type PolicyRow, type UsageKey } from "@/lib/api/edgequake/model-catalog";
import { groupByProvider, modelMatchesQuery } from "@/lib/settings/model-jobs";
import {
  disabledFromRows,
  narrowingMatrix,
  narrowingRowsFromDisabled,
  samePolicyRows,
  togglePair,
  USAGE_LABELS,
  type DisabledSelection,
  type NarrowingRow,
} from "@/lib/settings/model-usage";
import { ModelJobRail, type JobFilter } from "./model-job-rail";
import { ModelSaveBar, saveWithToast } from "./model-save-bar";

const NO_ROWS: readonly PolicyRow[] = [];

interface WorkspaceModelNarrowingCardProps {
  workspaceId: string;
  /** Members see the list; only editors can change it. */
  readOnly?: boolean;
}

function turnedOffSummary(count: number): string {
  return count === 0 ? "All on" : `${count} off`;
}

export function WorkspaceModelNarrowingCard({ workspaceId, readOnly = false }: WorkspaceModelNarrowingCardProps) {
  const policy = useWorkspaceModelPolicy(workspaceId);
  const eligible = useEligibleModelsByUsage(workspaceId);
  const save = useSaveWorkspaceModelPolicy(workspaceId);
  const [job, setJob] = useState<JobFilter>("all");
  const [query, setQuery] = useState("");

  const savedRows = policy.data?.rows ?? NO_ROWS;
  const { draft, setDraft, reset } = useServerBackedDraft(savedRows, disabledFromRows);
  const dirty = !samePolicyRows(narrowingRowsFromDisabled(draft), savedRows);
  const matrix = narrowingMatrix(eligible.byUsage, draft);
  const locked = readOnly || policy.data?.can_manage === false;
  const countOff = (usage: UsageKey) => (draft[usage] ?? []).length;

  const visible = useMemo(
    () =>
      matrix.filter((row) => {
        if (!modelMatchesQuery(`${row.providerLabel} ${row.label}`, query)) return false;
        if (job !== "all" && !row.presentIn.has(job)) return false;
        return true;
      }),
    [matrix, job, query],
  );
  const groups = useMemo(() => groupByProvider(visible, (row) => row.providerLabel), [visible]);

  return (
    <Card data-testid="workspace-model-narrowing-card">
      <CardHeader>
        <CardTitle>Narrow models for this workspace</CardTitle>
        <CardDescription>
          A ticked model stays available for that job. Untick one to keep this workspace from using
          it. Models the organization does not allow are not listed.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {eligible.loading || policy.isPending ? (
          <p className="text-sm text-muted-foreground">Loading models…</p>
        ) : matrix.length === 0 ? (
          <p className="text-sm text-muted-foreground">No models are available to this workspace yet.</p>
        ) : (
          <>
            <ModelJobRail
              active={job}
              onChange={setJob}
              query={query}
              onQuery={setQuery}
              summaryFor={(usage) => turnedOffSummary(countOff(usage))}
              searchId="narrowing-model-search"
            />
            {groups.length === 0 ? (
              <p className="text-sm text-muted-foreground">No model matches.</p>
            ) : (
              <div className="space-y-6" data-testid="workspace-narrowing-table">
                {groups.map((group) => (
                  <section key={group.provider} aria-label={group.provider} className="space-y-1">
                    <h3 className="text-sm font-medium text-muted-foreground">{group.provider}</h3>
                    <ul className="divide-y rounded-lg border">
                      {group.rows.map((row) => (
                        <NarrowingModelRow
                          key={row.value}
                          row={row}
                          job={job}
                          draft={draft}
                          locked={locked}
                          onToggle={(usage) => setDraft((current) => togglePair(current, usage, row.value))}
                        />
                      ))}
                    </ul>
                  </section>
                ))}
              </div>
            )}
          </>
        )}
        {locked ? (
          <p role="note" className="text-sm text-muted-foreground">
            Only workspace owners and admins can narrow these models.
          </p>
        ) : (
          <ModelSaveBar
            dirty={dirty}
            saving={save.isPending}
            onDiscard={reset}
            onSave={() =>
              void saveWithToast(() => save.mutateAsync(narrowingRowsFromDisabled(draft)), "Workspace models saved")
            }
          />
        )}
      </CardContent>
    </Card>
  );
}

function NarrowingModelRow({
  row,
  job,
  draft,
  locked,
  onToggle,
}: {
  row: NarrowingRow;
  job: JobFilter;
  draft: DisabledSelection;
  locked: boolean;
  onToggle: (usage: UsageKey) => void;
}) {
  const usages = USAGE_KEYS.filter((usage) => row.presentIn.has(usage) && (job === "all" || usage === job));
  return (
    <li
      data-testid={`narrowing-row-${row.value}`}
      className="flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-start sm:justify-between"
    >
      <div className="min-w-0 font-medium">{row.label}</div>
      <div className="flex flex-wrap gap-2">
        {usages.map((usage) => {
          const allowed = !(draft[usage] ?? []).includes(row.value);
          return (
            <label
              key={usage}
              className="inline-flex items-center gap-2 rounded-md border px-2 py-1 text-xs"
            >
              <Checkbox
                aria-label={`Use ${row.label} for ${USAGE_LABELS[usage]}`}
                checked={allowed}
                disabled={locked}
                onCheckedChange={() => onToggle(usage)}
              />
              {USAGE_LABELS[usage]}
            </label>
          );
        })}
      </div>
    </li>
  );
}
