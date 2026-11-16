"use client";

/**
 * SPEC-175 W5 — the organization's model ceiling.
 *
 * Each job starts open: any registered model that can do the job may run it. Ticking a model
 * closes that job to the models you allow. Workspaces can narrow a closed job and cannot widen it.
 * The list is by job, then by provider, because a grid of empty boxes reads as "nothing allowed".
 */

import { useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
  useSaveTenantModelPolicy,
  useTenantModelCatalog,
  useTenantModelPolicy,
} from "@/hooks/use-model-policy";
import { useServerBackedDraft } from "@/hooks/use-server-backed-draft";
import type { UsageKey, PolicyRow } from "@/lib/api/edgequake/model-catalog";
import { groupByProvider, isMovingAlias, modelMatchesQuery } from "@/lib/settings/model-jobs";
import {
  ceilingDisplayRows,
  ceilingRowsFromSelection,
  samePolicyRows,
  selectionFromCeilingRows,
  togglePair,
  USAGE_LABELS,
  type CeilingDisplayRow,
  type PairSelection,
} from "@/lib/settings/model-usage";
import { ModelJobRail, type JobFilter } from "./model-job-rail";
import { ModelSaveBar, saveWithToast } from "./model-save-bar";

const NO_ROWS: readonly PolicyRow[] = [];
const NO_SELECTION: PairSelection = {};

/** Soft tints, so a failed probe reads as a warning rather than a block of alarm colour. */
const HEALTH_BADGE: Record<
  Exclude<NonNullable<CeilingDisplayRow["health"]>, "healthy">,
  { label: string; variant: "secondary" | "outline"; className?: string }
> = {
  probe_failed: {
    label: "Check failed",
    variant: "outline",
    className: "border-destructive/30 bg-destructive/10 text-destructive",
  },
  unknown: { label: "Not checked", variant: "outline", className: "text-muted-foreground" },
};

interface TenantModelCeilingCardProps {
  tenantId: string;
}

/** How many models a usage allows, in words. An open job says "Any model". */
function allowedSummary(count: number): string {
  return count === 0 ? "Any model" : `${count} allowed`;
}

export function TenantModelCeilingCard({ tenantId }: TenantModelCeilingCardProps) {
  const catalog = useTenantModelCatalog(tenantId);
  const policy = useTenantModelPolicy(tenantId);
  const save = useSaveTenantModelPolicy(tenantId);
  const [job, setJob] = useState<JobFilter>("all");
  const [query, setQuery] = useState("");

  const savedRows = policy.data?.rows ?? NO_ROWS;
  const { draft, setDraft, reset } = useServerBackedDraft(savedRows, selectionFromCeilingRows);
  const dirty = !samePolicyRows(ceilingRowsFromSelection(draft), savedRows);
  const displayRows = ceilingDisplayRows(catalog.data?.entries ?? [], draft);
  const countFor = (usage: UsageKey) => (draft[usage] ?? []).length;
  const limited = (Object.keys(draft) as UsageKey[]).filter((usage) => countFor(usage) > 0);

  const visible = useMemo(
    () =>
      displayRows.filter((row) => {
        if (!modelMatchesQuery(`${row.providerLabel} ${row.modelLabel}`, query)) return false;
        if (job !== "all" && !row.usages.includes(job)) return false;
        return true;
      }),
    [displayRows, job, query],
  );
  const groups = useMemo(() => groupByProvider(visible, (row) => row.providerLabel), [visible]);

  return (
    <Card data-testid="tenant-model-ceiling-card">
      <CardHeader>
        <CardTitle>Model ceiling</CardTitle>
        <CardDescription>
          The most this organization allows for each job. Workspaces can narrow a job, and they
          cannot widen it. A job you have not limited can use any registered model that is able to
          do it.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {catalog.isPending ? (
          <p className="text-sm text-muted-foreground">Loading registered models…</p>
        ) : displayRows.length === 0 ? (
          <p className="text-sm text-muted-foreground">No models are registered on this deployment.</p>
        ) : (
          <>
            <ModelJobRail
              active={job}
              onChange={setJob}
              query={query}
              onQuery={setQuery}
              summaryFor={(usage) => allowedSummary(countFor(usage))}
              summaryTestId={(usage) => `ceiling-summary-${usage}`}
              searchId="ceiling-model-search"
            />
            {limited.length > 0 ? (
              <p className="text-sm" role="status">
                {limited.map((usage) => USAGE_LABELS[usage]).join(", ")}{" "}
                {limited.length === 1 ? "allows" : "allow"} only the models ticked for{" "}
                {limited.length === 1 ? "it" : "them"}. An unticked job stays open.
              </p>
            ) : (
              <p className="text-sm text-muted-foreground">
                Every job is open. The first model you allow for a job closes that job to the models
                you allow.
              </p>
            )}
            {job === "embedding" ? (
              <p className="text-sm text-muted-foreground">
                Vectors have a fixed width. A workspace that already stored vectors keeps that width
                until someone rebuilds them.
              </p>
            ) : null}
            {groups.length === 0 ? (
              <p className="text-sm text-muted-foreground">No model matches.</p>
            ) : (
              <div className="space-y-6" data-testid="tenant-model-ceiling-table">
                {groups.map((group) => (
                  <section key={group.provider} aria-label={group.provider} className="space-y-1">
                    <h3 className="text-sm font-medium text-muted-foreground">{group.provider}</h3>
                    <ul className="divide-y rounded-lg border">
                      {group.rows.map((row) => (
                        <CeilingModelRow
                          key={row.key}
                          row={row}
                          job={job}
                          draft={draft}
                          onToggle={(usage) => setDraft((current) => togglePair(current, usage, row.key))}
                        />
                      ))}
                    </ul>
                  </section>
                ))}
              </div>
            )}
          </>
        )}
        <div className="flex items-center justify-between gap-2">
          <Button
            type="button"
            variant="outline"
            disabled={save.isPending || ceilingRowsFromSelection(draft).length === 0}
            onClick={() => setDraft(() => NO_SELECTION)}
          >
            Clear all limits
          </Button>
          <ModelSaveBar
            dirty={dirty}
            saving={save.isPending}
            onDiscard={reset}
            onSave={() =>
              void saveWithToast(() => save.mutateAsync(ceilingRowsFromSelection(draft)), "Model ceiling saved")
            }
          />
        </div>
      </CardContent>
    </Card>
  );
}

function CeilingModelRow({
  row,
  job,
  draft,
  onToggle,
}: {
  row: CeilingDisplayRow;
  job: JobFilter;
  draft: PairSelection;
  onToggle: (usage: UsageKey) => void;
}) {
  const usages = job === "all" ? row.usages : row.usages.filter((usage) => usage === job);
  const health = row.health && row.health !== "healthy" ? HEALTH_BADGE[row.health] : null;
  return (
    <li
      data-testid={`ceiling-row-${row.key}`}
      className="flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-start sm:justify-between"
    >
      <div className="min-w-0 space-y-1">
        <div className="font-medium">{row.modelLabel}</div>
        {isMovingAlias(`${row.modelLabel} ${row.label}`) ? (
          <p className="max-w-md text-xs text-muted-foreground">
            This name follows whichever model is loaded in the app. A limit here changes when that
            load changes.
          </p>
        ) : null}
        {health ? (
          <Badge variant={health.variant} className={health.className}>
            {health.label}
          </Badge>
        ) : null}
        {!row.registered ? <Badge variant="outline">Not registered</Badge> : null}
      </div>
      <div className="flex flex-wrap gap-2">
        {usages.map((usage) => {
          const checked = (draft[usage] ?? []).includes(row.key);
          return (
            <label
              key={usage}
              className="inline-flex items-center gap-2 rounded-md border px-2 py-1 text-xs"
            >
              <Checkbox
                aria-label={`Allow ${row.label} for ${USAGE_LABELS[usage]}`}
                checked={checked}
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
