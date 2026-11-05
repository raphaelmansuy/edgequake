"use client";

/**
 * SPEC-175 W4 — the typed usage editor for workspace roles (`llm_roles`). Replaces the free-text
 * role matrix. Each row offers only models the server marks eligible for that usage, and shows
 * what the ladder resolved, including locked and broken states.
 */

import { useCallback, useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useConnectionOptions, useEffectiveUsages } from "@/hooks/use-model-policy";
import { useWorkspaceLlmRoles } from "@/hooks/use-workspace-llm-roles";
import { USAGE_KEYS, type UsageKey } from "@/lib/api/edgequake/model-catalog";
import { WRITABLE_ROLE_USAGES, type RoleDraft } from "@/lib/settings/model-usage";
import { useTenantStore } from "@/stores/use-tenant-store";
import { Grid3x3 } from "lucide-react";
import { ModelSaveBar, saveWithToast } from "./model-save-bar";
import { UsageRoleRow } from "./usage-role-row";

const EMPTY_ROLE: RoleDraft = { provider: "", model: "", connection_id: "" };
const WRITABLE: ReadonlySet<UsageKey> = new Set(WRITABLE_ROLE_USAGES);

interface WorkspaceRoleEditorProps {
  /** Members read the rows; only workspace editors see the controls and the save bar. */
  readOnly?: boolean;
}

export function WorkspaceRoleEditor({ readOnly = false }: WorkspaceRoleEditorProps) {
  const selectedWorkspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const workspaces = useTenantStore((s) => s.workspaces);
  const workspace = workspaces.find((w) => w.id === selectedWorkspaceId) ?? null;

  const effective = useEffectiveUsages(workspace?.id ?? null);
  const connections = useConnectionOptions();
  const roles = useWorkspaceLlmRoles(workspace);

  const resolved = new Map((effective.data?.usages ?? []).map((u) => [u.usage, u]));
  const connectionOptions = connections.data ?? [];
  /*
   * SPEC-184 — the count comes from the rows, not from a second derivation here.
   *
   * A row flags a saved model for two different reasons (the server resolved it as broken, or the
   * picker cannot find it among the eligible ones). Counting only the first said "3" while four
   * rows were red. Each row now reports its own verdict, so the summary and the table cannot
   * disagree.
   */
  const [flaggedUsages, setFlaggedUsages] = useState<ReadonlySet<UsageKey>>(() => new Set());
  const reportFlag = useCallback((usage: UsageKey, flagged: boolean) => {
    setFlaggedUsages((current) => {
      const has = current.has(usage);
      if (has === flagged) return current;
      const next = new Set(current);
      if (flagged) next.add(usage);
      else next.delete(usage);
      return next;
    });
  }, []);
  const ineligibleCount = flaggedUsages.size;

  return (
    <Card data-testid="model-roles-card">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Grid3x3 className="h-5 w-5" />
          Model roles
        </CardTitle>
        <CardDescription>
          Pick the model each job uses. Only models allowed for this workspace are listed. An empty
          model inherits the workspace default.
        </CardDescription>
        {/*
          SPEC-184 — the "pick another one to save" instruction is identical for every affected
          row, so it belongs here once with a count instead of repeated four times inside the
          table. The rows keep their own short reason, which is the part that differs.
        */}
        {ineligibleCount > 0 ? (
          <p className="text-xs text-destructive" role="status" data-testid="model-roles-ineligible-summary">
            {ineligibleCount === 1
              ? "1 job cannot use its saved model here. Pick another one to save."
              : `${ineligibleCount} jobs cannot use their saved model here. Pick another one for each to save.`}
          </p>
        ) : null}
      </CardHeader>
      <CardContent className="space-y-4">
        {!workspace ? (
          <p className="text-sm text-muted-foreground">Select a workspace to see its models.</p>
        ) : (
          <>
            {/* Column names once, so seven stacked rows read as one table. Hidden when stacked. */}
            <div
              aria-hidden="true"
              className="hidden border-b px-0 pb-2 text-xs font-medium text-muted-foreground md:grid md:grid-cols-[minmax(0,1.1fr)_minmax(0,1.4fr)_minmax(0,1fr)] md:gap-3"
            >
              <span>Job</span>
              <span>Model</span>
              <span>Connection</span>
            </div>
            <div className="divide-y" data-testid="model-roles-rows">
              {USAGE_KEYS.map((usage) => (
                <UsageRoleRow
                  key={usage}
                  usage={usage}
                  workspaceId={workspace.id}
                  draft={WRITABLE.has(usage) ? (roles.draft[usage] ?? EMPTY_ROLE) : undefined}
                  resolved={resolved.get(usage)}
                  connections={connectionOptions}
                  readOnly={readOnly}
                  onChange={(next) => roles.patchUsage(usage, next)}
                  onFlagChange={reportFlag}
                />
              ))}
            </div>
          </>
        )}
        {readOnly ? (
          <p role="note" data-testid="model-roles-readonly" className="text-sm text-muted-foreground">
            Only workspace owners and admins can change these models.
          </p>
        ) : workspace ? (
          <ModelSaveBar
            dirty={roles.dirty}
            saving={roles.saving}
            onDiscard={roles.reset}
            onSave={() => void saveWithToast(() => roles.save(), "Model roles saved")}
          />
        ) : null}
      </CardContent>
    </Card>
  );
}
