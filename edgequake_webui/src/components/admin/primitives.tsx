"use client";

/**
 * SPEC-181 — the four primitives the admin surfaces are built from.
 *
 * Borrowed semantically from gr8r-studio (`MeterRow`, the segmented control, the capability
 * table) and mapped onto the tokens EdgeQuake already has, so there is no new stylesheet and
 * no literal colour. `SettingRow` already exists in `settings-frame.tsx` and is re-exported here
 * so an admin screen imports its primitives from one place.
 */

import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

export { SettingRow, SettingsGroup, SettingsNotice } from "@/components/settings/shell/settings-frame";

/**
 * A plan budget: `3 of 10 workspaces` plus a bar.
 *
 * The numbers are text and the bar is decorative, so a screen reader hears the sentence and a
 * monochrome screenshot still carries the information.
 */
export function MeterRow({
  label,
  used,
  limit,
  className,
  testId,
}: {
  label: string;
  used: number;
  limit: number;
  className?: string;
  testId?: string;
}) {
  // `limit === 0` is "unlimited": no bar, and the copy says so rather than dividing by zero.
  const unlimited = limit === 0;
  const ratio = unlimited ? 0 : Math.min(1, used / limit);
  const tone = ratio >= 1 ? "bg-destructive" : ratio >= 0.8 ? "bg-amber-500" : "bg-primary";

  return (
    <div className={cn("space-y-1.5", className)} data-testid={testId}>
      <div className="flex items-baseline justify-between gap-3 text-[11px] font-medium uppercase tracking-[0.06em] text-muted-foreground">
        <span>{label}</span>
        <span className={cn("normal-case tracking-normal tabular-nums", ratio >= 1 && "text-destructive")}>
          {unlimited ? `${used} · unlimited` : `${used} of ${limit}`}
        </span>
      </div>
      {!unlimited && (
        <div aria-hidden className="h-1.5 overflow-hidden rounded-full bg-muted">
          <div
            className={cn("h-full rounded-full transition-[width] duration-200", tone)}
            style={{ width: `${Math.max(ratio * 100, used > 0 ? 4 : 0)}%` }}
          />
        </div>
      )}
    </div>
  );
}

/** 2–4 exclusive options on one line. Terse enums only; consequence text uses radio cards. */
export function SegmentedControl<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
  disabled,
  testId,
}: {
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (next: T) => void;
  ariaLabel: string;
  disabled?: boolean;
  testId?: string;
}) {
  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      data-testid={testId}
      className="inline-flex rounded-md border bg-card p-0.5"
    >
      {options.map((option) => {
        const active = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={active}
            disabled={disabled}
            onClick={() => onChange(option.value)}
            data-testid={testId ? `${testId}-${option.value}` : undefined}
            className={cn(
              "h-6 rounded-[5px] px-2.5 text-[12.5px] font-medium transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              active
                ? "bg-card text-foreground shadow-[0_1px_2px_rgb(0_0_0/0.05),0_0_0_1px_var(--border)]"
                : "text-muted-foreground hover:text-foreground",
              disabled && "cursor-not-allowed opacity-50",
            )}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

/**
 * `Inherited from Acme · vision`, or the value when this scope set it.
 *
 * LAW-181-13: wherever a value can be inherited, the layer and the value are printed, so
 * "inherit" is a visible decision rather than an absence.
 */
export function InheritLine({
  layer,
  value,
  className,
}: {
  /** `Acme`, `server defaults`, `this workspace` … */
  layer: string | null;
  value: ReactNode;
  className?: string;
}) {
  return (
    <span className={cn("inline-flex min-w-0 flex-wrap items-baseline gap-1.5 text-[12.5px]", className)}>
      {layer ? (
        <span className="text-muted-foreground">
          Inherited from <span className="text-foreground">{layer}</span>
        </span>
      ) : (
        <span className="text-muted-foreground">Not set</span>
      )}
      <span aria-hidden className="text-muted-foreground/50">·</span>
      <span className="min-w-0 truncate font-mono text-[12px] text-foreground">{value}</span>
    </span>
  );
}

/**
 * Capability × role, grouped by axis.
 *
 * Rendered from `GET /roles`, which derives every row from the predicates the handlers call, so
 * the matrix cannot claim a right the API refuses. Workspace capabilities appear under their own
 * columns, never mixed into the organization block.
 */
export function CapabilityTable({
  capabilities,
  roles,
  roleLabels,
  testId,
}: {
  capabilities: readonly {
    id: string;
    axis: string;
    group: string;
    label: string;
    roles: string[];
  }[];
  /** Role ids in column order, highest privilege first. */
  roles: readonly string[];
  roleLabels: Record<string, string>;
  testId?: string;
}) {
  const groups = new Map<string, typeof capabilities>();
  for (const cap of capabilities) {
    const key = `${cap.axis}:${cap.group}`;
    const list = groups.get(key) ?? [];
    groups.set(key, [...list, cap]);
  }

  return (
    <div className="overflow-x-auto rounded-lg border bg-card" data-testid={testId}>
      <table className="w-full min-w-[34rem] text-[12.5px]">
        <thead className="bg-muted/30">
          <tr>
            <th scope="col" className="px-3 py-2 text-left font-medium uppercase tracking-[0.06em] text-[11px] text-muted-foreground">
              Capability
            </th>
            {roles.map((role) => (
              <th key={role} scope="col" className="w-24 px-2 py-2 text-center text-[12px] font-semibold">
                {roleLabels[role] ?? role}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {[...groups.entries()].map(([key, rows]) => {
            const axis = key.split(":")[0];
            return (
              <>
                <tr key={`group-${key}`} className="border-t bg-muted/20">
                  <th
                    colSpan={roles.length + 1}
                    scope="colgroup"
                    className="px-3 py-1 text-left text-[10.5px] font-semibold uppercase tracking-[0.07em] text-muted-foreground"
                  >
                    {axis === "workspace" ? "Workspace" : axis === "organization" ? "Organization" : axis}
                  </th>
                </tr>
                {rows.map((cap) => (
                  <tr key={cap.id} className="border-t" data-testid={`cap-${cap.id}`}>
                    <th scope="row" className="px-3 py-1.5 text-left font-normal">
                      {cap.label}
                    </th>
                    {roles.map((role) => {
                      const granted = cap.roles.includes(role);
                      return (
                        <td key={role} className="px-2 py-1.5 text-center" data-granted={granted}>
                          {granted ? (
                            <span className="text-success" aria-label="granted">✓</span>
                          ) : (
                            <span className="text-muted-foreground/40" aria-label="not granted">—</span>
                          )}
                        </td>
                      );
                    })}
                  </tr>
                ))}
              </>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
