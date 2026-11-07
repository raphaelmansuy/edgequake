"use client";

/**
 * The jobs a model can be limited for, as a choice of one job (or all of them).
 * The summary under each name is the caller's words, so the ceiling and the
 * workspace list can describe "allowed" and "turned off" differently.
 */

import { Input } from "@/components/ui/input";
import { USAGE_KEYS, type UsageKey } from "@/lib/api/edgequake/model-catalog";
import { JOB_PURPOSE } from "@/lib/settings/model-jobs";
import { USAGE_LABELS } from "@/lib/settings/model-usage";
import { cn } from "@/lib/utils";

export type JobFilter = UsageKey | "all";

interface ModelJobRailProps {
  active: JobFilter;
  onChange: (next: JobFilter) => void;
  query: string;
  onQuery: (next: string) => void;
  summaryFor: (usage: UsageKey) => string;
  /** When set, the summary span carries this test id. The ceiling tests read it. */
  summaryTestId?: (usage: UsageKey) => string;
  searchId: string;
}

export function ModelJobRail({
  active,
  onChange,
  query,
  onQuery,
  summaryFor,
  summaryTestId,
  searchId,
}: ModelJobRailProps) {
  const purpose = active === "all"
    ? "Each model lists only the jobs it can do. Choose a job to work on that list alone."
    : JOB_PURPOSE[active];

  return (
    <div className="space-y-3">
      <div role="tablist" aria-label="Jobs" className="flex flex-wrap gap-2">
        <JobTab selected={active === "all"} onSelect={() => onChange("all")} label="All jobs" detail="Every model" />
        {USAGE_KEYS.map((usage) => (
          <JobTab
            key={usage}
            selected={active === usage}
            onSelect={() => onChange(usage)}
            label={USAGE_LABELS[usage]}
            detail={summaryFor(usage)}
            detailTestId={summaryTestId?.(usage)}
          />
        ))}
      </div>
      <p className="text-sm text-muted-foreground" data-testid="job-purpose">
        {purpose}
      </p>
      <div className="max-w-sm">
        <label htmlFor={searchId} className="sr-only">
          Find a model
        </label>
        <Input
          id={searchId}
          value={query}
          onChange={(event) => onQuery(event.target.value)}
          placeholder="Find a model"
          autoComplete="off"
        />
      </div>
    </div>
  );
}

function JobTab({
  selected,
  onSelect,
  label,
  detail,
  detailTestId,
}: {
  selected: boolean;
  onSelect: () => void;
  label: string;
  detail: string;
  detailTestId?: string;
}) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={selected}
      onClick={onSelect}
      className={cn(
        "flex min-w-40 flex-1 basis-40 flex-col items-start rounded-lg border px-3 py-2 text-left transition-colors",
        "focus-visible:border-ring focus-visible:ring-ring focus-visible:ring-2 focus-visible:outline-none",
        selected ? "border-ring bg-accent text-accent-foreground" : "hover:bg-accent/50",
      )}
    >
      <span className="text-sm font-medium">{label}</span>
      <span
        data-testid={detailTestId}
        className={cn("text-xs", selected ? "text-accent-foreground/80" : "text-muted-foreground")}
      >
        {detail}
      </span>
    </button>
  );
}
