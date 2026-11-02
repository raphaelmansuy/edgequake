"use client";

/**
 * SPEC-181 W-03 — one model role in the Create workspace dialog.
 *
 * A collapsed line states what will run; opening it mounts the same two-step picker the
 * workspace settings page uses, so nothing about choosing a provider or a model is
 * re-implemented here. The row is a button, not a clickable div, so the whole line is one
 * keyboard stop and `aria-expanded` says what it does.
 */

import { ProviderIcon } from "@/components/providers/provider-icon";
import { Button } from "@/components/ui/button";
import {
  EmbeddingModelSelector,
  type EmbeddingSelection,
} from "@/components/workspace/embedding-model-selector";
import { LLMModelSelector } from "@/components/workspace/llm-model-selector";
import {
  ROLE_BLURB,
  ROLE_LABEL,
  verdictSentence,
  type ModelPickLike,
  type ModelRole,
  type RoleSummary,
  type RoleVerdict,
} from "@/lib/admin/workspace-model-draft";
import { cn } from "@/lib/utils";
import { AlertCircle, ChevronDown } from "lucide-react";
import { useId } from "react";

/**
 * The pick a row may hold. Both pickers return this shape (`LLMSelection`,
 * `EmbeddingSelection`), so one callback serves all three roles without a cast.
 */
export interface RoleSelection extends ModelPickLike {
  provider: string;
  model: string;
}

export interface ModelRoleRowProps {
  role: ModelRole;
  summary: RoleSummary;
  open: boolean;
  onToggle: () => void;
  disabled?: boolean;
  verdict?: RoleVerdict;
  pick: ModelPickLike | undefined;
  onPick: (next: RoleSelection | undefined) => void;
  /** The tenant the workspace is being created in; the pickers read its offer. */
  tenantId: string | null;
  /** How the inherited pair is named in the picker ("Organization default (openai/…)"). */
  inheritLabel: string;
  inheritPair: { provider?: string; model?: string };
}

export function ModelRoleRow({
  role,
  summary,
  open,
  onToggle,
  disabled = false,
  verdict,
  pick,
  onPick,
  tenantId,
  inheritLabel,
  inheritPair,
}: ModelRoleRowProps) {
  const panelId = useId();
  const label = ROLE_LABEL[role];
  /**
   * A refusal reads two ways, and they are not the same severity. A pinned pair the server would
   * reject blocks Create, so it is destructive. An unpinned row inherits, and the server accepts
   * an inheriting create — the stale default is a warning, not a wall.
   */
  const pinned = Boolean(pick?.provider?.trim() && pick?.model?.trim());
  const refusal = verdictSentence(verdict);
  const hardRefusal = pinned && Boolean(refusal);
  const advisory = !pinned && Boolean(refusal);

  return (
    <div
      data-wizard-model-role={role}
      data-testid={`create-workspace-models-row-${role}`}
      data-open={open ? "true" : "false"}
      data-refusal={hardRefusal ? "blocking" : advisory ? "advisory" : "none"}
      className={cn(
        "overflow-hidden rounded-lg border transition-colors",
        hardRefusal
          ? "border-destructive/60"
          : advisory
            ? "border-amber-500/50"
            : "border-border",
        open && "bg-muted/[0.18]",
      )}
    >
      <button
        type="button"
        onClick={onToggle}
        disabled={disabled}
        aria-expanded={open}
        aria-controls={panelId}
        data-testid={`create-workspace-models-row-${role}-toggle`}
        className={cn(
          "flex w-full items-center gap-2.5 px-3 py-2 text-left transition-colors",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset",
          !disabled && "hover:bg-muted/40",
          disabled && "cursor-not-allowed opacity-60",
        )}
      >
        <span className="w-[3.75rem] shrink-0 text-[12px] font-medium text-muted-foreground">
          {label}
        </span>
        <ProviderIcon
          providerId={summary.chip === "workspace" ? pick?.provider : inheritPair.provider}
        />
        <span className="min-w-0 flex-1">
          <span className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5">
            <span
              className={cn(
                "truncate text-[13px]",
                summary.unset ? "text-muted-foreground" : "font-medium text-foreground",
              )}
            >
              {summary.title}
            </span>
            <Chip kind={summary.chip} />
            {summary.detail ? (
              <span className="shrink-0 text-[11.5px] tabular-nums text-muted-foreground">
                {summary.detail}
              </span>
            ) : null}
          </span>
          {summary.id ? (
            <span className="mt-0.5 block truncate font-mono text-[11px] text-muted-foreground">
              {summary.id}
            </span>
          ) : null}
        </span>
        <ChevronDown
          aria-hidden
          className={cn(
            "h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform duration-150 motion-reduce:transition-none",
            open && "rotate-180",
          )}
        />
      </button>

      {open ? (
        <div
          id={panelId}
          data-testid={`create-workspace-models-row-${role}-picker`}
          className={cn(
            "space-y-2 border-t px-3 py-3",
            // Motion is a courtesy, never a requirement: honour the system preference.
            "motion-safe:animate-in motion-safe:fade-in-0 motion-safe:slide-in-from-top-1 motion-safe:duration-150",
          )}
        >
          <p className="text-[11.5px] leading-snug text-muted-foreground">{ROLE_BLURB[role]}</p>
          {role === "embedding" ? (
            <EmbeddingModelSelector
              value={
                pick?.provider && pick.model
                  ? {
                      provider: pick.provider,
                      model: pick.model,
                      dimension: pick.dimension ?? 0,
                    }
                  : undefined
              }
              onChange={(next: EmbeddingSelection | undefined) => onPick(next)}
              choiceScope="tenant"
              scopeId={tenantId}
              inheritLabel={inheritLabel}
              inheritPair={inheritPair}
            />
          ) : (
            <LLMModelSelector
              value={
                pick?.provider && pick.model
                  ? {
                      provider: pick.provider,
                      model: pick.model,
                      fullId: `${pick.provider}/${pick.model}`,
                    }
                  : undefined
              }
              onChange={(next) => onPick(next)}
              choiceScope="tenant"
              scopeId={tenantId}
              filterVision={role === "vision"}
              usage={role === "vision" ? "vlm" : "extract"}
              showUsageHint={false}
              showCapabilityFilters={false}
              inheritLabel={inheritLabel}
              inheritPair={inheritPair}
            />
          )}
          {refusal ? (
            <p
              role={hardRefusal ? "alert" : "status"}
              data-testid={`create-workspace-models-row-${role}-refusal`}
              className={cn(
                "flex items-start gap-1.5 text-[11.5px]",
                hardRefusal ? "text-destructive" : "text-amber-600 dark:text-amber-400",
              )}
            >
              <AlertCircle aria-hidden className="mt-px h-3.5 w-3.5 shrink-0" />
              <span>
                {refusal}
                {advisory ? " The organization default needs fixing." : null}
              </span>
            </p>
          ) : null}
          <div className="flex justify-end">
            {/* A row that can be changed but not re-inherited is a one-way door. */}
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-[11.5px] text-muted-foreground hover:text-foreground"
              onClick={() => onPick(undefined)}
              disabled={!pick?.provider && !pick?.model}
              data-testid={`create-workspace-models-row-${role}-clear`}
            >
              Use inherited
            </Button>
          </div>
        </div>
      ) : refusal ? (
        <p
          role={hardRefusal ? "alert" : "status"}
          data-testid={`create-workspace-models-row-${role}-refusal`}
          className={cn(
            "px-3 pb-2 text-[11.5px]",
            hardRefusal ? "text-destructive" : "text-amber-600 dark:text-amber-400",
          )}
        >
          {refusal}
        </p>
      ) : null}
    </div>
  );
}

/** Where the pair comes from. Three words, not a sentence: the row is read at a glance. */
function Chip({ kind }: { kind: RoleSummary["chip"] }) {
  if (kind === "follows") {
    return (
      <span className="shrink-0 rounded-full bg-muted px-1.5 py-0.5 text-[10.5px] font-medium text-muted-foreground">
        follows Extract
      </span>
    );
  }
  if (kind === "workspace") {
    return (
      <span className="shrink-0 rounded-full bg-primary/10 px-1.5 py-0.5 text-[10.5px] font-medium text-primary">
        this workspace
      </span>
    );
  }
  return (
    <span className="shrink-0 rounded-full bg-muted px-1.5 py-0.5 text-[10.5px] font-medium text-muted-foreground">
      inherited
    </span>
  );
}
