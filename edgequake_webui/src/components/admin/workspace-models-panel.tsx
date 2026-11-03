"use client";

/**
 * SPEC-181 W-03 — the Models decision in the Create workspace dialog.
 *
 * The pre-181 build printed three inherited ids in a grey box with no way to change them, so the
 * only way to a non-default model was to create the workspace and edit it afterwards (which
 * re-indexes). This panel makes inheritance a visible default and the alternative a first-class
 * choice — two options, each with its consequence stated, in the same card language the
 * retrieval decision already uses.
 *
 * It owns no state: the draft lives in the dialog so the POST body and the "dirty" signal have
 * one source (LAW-181-14).
 */

import { OptionCard } from "@/components/onboarding/option-card";
import { ModelRoleRow, type RoleSelection } from "@/components/admin/model-role-row";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  blockedModelSentence,
  inheritedRoles,
  MODEL_ROLES,
  pickedCount,
  refusedPickedRoles,
  refusedRoles,
  ROLE_LABEL,
  roleSummary,
  type ModelRole,
  type RoleVerdict,
  type WorkspaceModelDraft,
} from "@/lib/admin/workspace-model-draft";
import { cn } from "@/lib/utils";
import { sectionHref } from "@/lib/settings/registry";
import Link from "next/link";
import { useMemo } from "react";

export interface WorkspaceModelsPanelProps {
  draft: WorkspaceModelDraft;
  onChange: (next: WorkspaceModelDraft) => void;
  /** The tenant whose offer the pickers read; null before the scope is known. */
  tenantId: string | null;
  /** "Acme Research" — printed in the inherit title, so inheritance names its layer. */
  orgName: string;
  isLoading: boolean;
  ladder: ReturnType<typeof inheritedRoles>;
  verdicts?: Partial<Record<ModelRole, RoleVerdict>>;
  /** Which row is open. The dialog owns it so "choose another" can open a specific row. */
  openRole: ModelRole | null;
  onOpenRoleChange: (role: ModelRole | null) => void;
  /** Organization admins can open the provider offer from the advisory; everyone else is told who can. */
  canManageProviders?: boolean;
  /** SPEC-183 — the workspace will store images, so the embedder must read them. */
  imageEmbeddingOnly?: boolean;
}

export function WorkspaceModelsPanel({
  draft,
  onChange,
  tenantId,
  orgName,
  isLoading,
  ladder,
  verdicts,
  openRole,
  onOpenRoleChange,
  canManageProviders = false,
  imageEmbeddingOnly = false,
}: WorkspaceModelsPanelProps) {
  const count = pickedCount(draft);
  const choosing = draft.source === "choose";
  /**
   * Only a pinned role can stop a create: the server judges the pairs the body names, and an
   * absent role inherits unjudged. So a refusal decorates a row only when that row is pinned —
   * a red border beside an enabled Create button states two contradictory things.
   */
  const blockedRoles = useMemo(() => refusedPickedRoles(verdicts, draft), [verdicts, draft]);
  /**
   * An inherited default that the offer no longer contains still deserves a word: it will break
   * ingest later. It is advisory, because the server accepts an inheriting create and a later
   * default change may fix it.
   */
  const staleInherited = useMemo(
    () => (choosing ? [] : refusedRoles(verdicts)),
    [verdicts, choosing],
  );
  /** The rows show what is in effect, so they read the pick when there is one. */
  const summaries = useMemo(
    () =>
      MODEL_ROLES.map((role) => ({ role, summary: roleSummary(role, ladder, draft[role]) })),
    [ladder, draft],
  );

  /**
   * The inherit card always describes the *inherited* ladder, never a pick. It is the option that
   * says "not mine", so showing the picked pair there would be a lie about what it does — and the
   * two cards would print the same value.
   */
  const inheritedSummaries = useMemo(
    () => MODEL_ROLES.map((role) => ({ role, summary: roleSummary(role, ladder, undefined) })),
    [ladder],
  );

  const setPick = (role: ModelRole, pick: RoleSelection | undefined) =>
    onChange({ ...draft, [role]: pick });

  /**
   * Choosing a complete pair closes the row: the result belongs to the summary line, and leaving
   * the picker open hides it. Clearing keeps the row open, because the next move is to pick again.
   */
  const commitPick = (role: ModelRole, next: RoleSelection | undefined) => {
    setPick(role, next);
    if (next?.provider && next.model) onOpenRoleChange(null);
  };

  const useInherit = () => {
    onChange({ source: "inherit" });
    onOpenRoleChange(null);
  };

  const choose = () => {
    if (choosing) return;
    onChange({ ...draft, source: "choose" });
    // Straight into the first row: the person has just said they want to pick something.
    if (!openRole) onOpenRoleChange("llm");
  };

  return (
    <div className="space-y-1.5">
      <div className="flex items-baseline justify-between gap-2">
        <Label className="text-[13px] font-medium">Models</Label>
        {choosing ? (
          <button
            type="button"
            onClick={useInherit}
            data-testid="create-workspace-models-use-inherited"
            className="rounded text-[11.5px] font-medium text-muted-foreground underline-offset-2 hover:text-foreground hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            {count > 0 ? `Reset ${count} choice${count === 1 ? "" : "s"}` : "Back to inherited"}
          </button>
        ) : (
          <span className="text-[11.5px] text-muted-foreground">all inherited</span>
        )}
      </div>

      <div className="space-y-2" data-testid="create-workspace-models">
        {/*
          Side by side, the idiom `ModelSourceChoice` already uses for this same decision:
          two cards stacked would push the alternative below the scrollport of a 648px dialog,
          and an alternative you have to scroll to find is not a decision.
        */}
        <div
          role="radiogroup"
          aria-label="Where do the models come from?"
          className="grid gap-2 md:grid-cols-5"
        >
          <InheritCard
            className="md:col-span-3"
            selected={!choosing}
            onSelect={useInherit}
            orgName={orgName}
            isLoading={isLoading}
            summaries={inheritedSummaries}
            onPickRole={(role) => {
              choose();
              onOpenRoleChange(role);
            }}
          />
          <OptionCard
            className="md:col-span-2"
            selected={choosing}
            onSelect={choose}
            title="Choose models"
            data-testid="create-workspace-models-choose"
          >
            <p className="text-[12px] leading-relaxed text-muted-foreground">
              Pin any role. The rest keeps following the organization.
            </p>
          </OptionCard>
        </div>

        {choosing ? (
          <div className="space-y-2 pt-1" data-testid="create-workspace-models-rows">
            {summaries.map(({ role, summary }) => (
              <ModelRoleRow
                key={role}
                role={role}
                summary={summary}
                open={openRole === role}
                onToggle={() => onOpenRoleChange(openRole === role ? null : role)}
                verdict={verdicts?.[role]}
                pick={draft[role]}
                onPick={(next) => commitPick(role, next)}
                tenantId={tenantId}
                inheritLabel={inheritLabelFor(role, ladder)}
                inheritPair={inheritPairFor(role, ladder)}
                imageEmbeddingOnly={imageEmbeddingOnly}
              />
            ))}
            <p className="text-[11.5px] leading-snug text-muted-foreground">
              The embedding model fixes the vector size, so changing it later means re-indexing
              every document. Extract and Vision can be changed at any time.
            </p>
          </div>
        ) : null}

        {choosing && blockedRoles.length > 0 ? (
          <p
            role="alert"
            data-testid="create-workspace-models-blocked"
            className="text-[11.5px] font-medium text-destructive"
          >
            {blockedModelSentence(blockedRoles)}
          </p>
        ) : null}

        {/* Advisory only: the server accepts an inheriting create, so this never gates Create. */}
        {!choosing && staleInherited.length > 0 ? (
          <div
            role="status"
            data-testid="create-workspace-models-stale-inherit"
            className="space-y-1"
          >
            <p className="text-[11.5px] leading-snug text-amber-600 dark:text-amber-400">
              The organization default for{" "}
              {staleInherited.map((role) => ROLE_LABEL[role]).join(" and ")} is not offered here
              any more. Ingest will fail until it is fixed, or you pin a model now.
            </p>
            {/* One fix, named where the reader already is (SPEC-179: explain, then point). */}
            {canManageProviders ? (
              <Button asChild variant="link" size="sm" className="h-auto p-0 text-[11.5px]">
                <Link
                  href={sectionHref("org-providers")}
                  data-testid="create-workspace-models-fix-providers"
                >
                  Fix organization providers →
                </Link>
              </Button>
            ) : (
              <p
                className="text-[11.5px] text-muted-foreground"
                data-testid="create-workspace-models-ask-admin"
              >
                Ask an organization admin to offer a provider again.
              </p>
            )}
          </div>
        ) : null}
      </div>
    </div>
  );
}

/**
 * The default. It keeps printing the three inherited ids — that promise is from 181 and does not
 * change — but each line is now a button, so "these are not mine" leads somewhere.
 */
function InheritCard({
  className,
  selected,
  onSelect,
  orgName,
  isLoading,
  summaries,
  onPickRole,
}: {
  className?: string;
  selected: boolean;
  onSelect: () => void;
  orgName: string;
  isLoading: boolean;
  summaries: { role: ModelRole; summary: ReturnType<typeof roleSummary> }[];
  onPickRole: (role: ModelRole) => void;
}) {
  return (
    <OptionCard
      className={className}
      selected={selected}
      onSelect={onSelect}
      title={`Inherit from ${orgName}`}
      badge={selected ? "Recommended" : undefined}
      data-testid="create-workspace-models-inherit"
    >
      <dl className="space-y-1">
        {summaries.map(({ role, summary }) => (
          <div key={role} className="flex items-baseline gap-2">
            <dt className="w-[3.75rem] shrink-0 text-[12px] text-muted-foreground">
              {ROLE_LABEL[role]}
            </dt>
            {/*
              SPEC-181 §09: `Extract  gpt-5-nano · openai`, `Embed  text-embedding-3-small ·
              openai · 1536`. The provider is part of the promise — an id alone does not say who
              serves it — so it stays on the same line rather than in a second one.
            */}
            <dd className="min-w-0 flex-1 text-right">
              <button
                type="button"
                onClick={() => onPickRole(role)}
                title={`Change ${ROLE_LABEL[role].toLowerCase()} for this workspace`}
                data-testid={`create-workspace-models-inherit-${role}`}
                className={cn(
                  "inline-flex max-w-full items-baseline justify-end gap-1 rounded underline-offset-2",
                  "hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                )}
              >
                <span
                  className={cn(
                    "truncate font-mono text-[11.5px]",
                    summary.unset || summary.chip === "follows"
                      ? "text-muted-foreground"
                      : "text-foreground",
                  )}
                >
                  {isLoading ? "loading…" : summary.label}
                </span>
              </button>
            </dd>
          </div>
        ))}
      </dl>
      <p className="text-[11.5px] leading-relaxed text-muted-foreground">
        A later change of the organization default reaches this workspace. Pinning a model here
        stops that for that role.
      </p>
    </OptionCard>
  );
}

function inheritPairFor(role: ModelRole, ladder: ReturnType<typeof inheritedRoles>) {
  const pair = ladder[role];
  return { provider: pair.provider, model: pair.model };
}

/** LAW-101-13: the picker's "use default" link names the pair it would restore. */
function inheritLabelFor(role: ModelRole, ladder: ReturnType<typeof inheritedRoles>): string {
  const pair = ladder[role];
  if (!pair.provider || !pair.model) {
    return "Organization default (not configured)";
  }
  return `Organization default (${pair.provider}/${pair.model})`;
}
