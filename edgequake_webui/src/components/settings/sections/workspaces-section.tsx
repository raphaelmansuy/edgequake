"use client";

/**
 * SPEC-181 W-06 — Organization / Workspaces.
 *
 * The list an organization owner needs when the plan is full: every workspace, with the archived
 * ones separated, and the two actions that matter (create, archive). Before this pack there was
 * no such page — the only way to see the organization's workspaces was the header switcher.
 */

import { CreateWorkspaceDialog } from "@/components/admin/create-workspace-dialog";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { SegmentedControl } from "@/components/admin/primitives";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useAccess } from "@/lib/access/use-access";
import { archiveWorkspace, getWorkspaces, restoreWorkspace } from "@/lib/api/edgequake";
import { useTenantStore } from "@/stores/use-tenant-store";
import type { Workspace } from "@/types";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, Plus, RotateCcw } from "lucide-react";
import Link from "next/link";
import { useMemo, useState } from "react";
import { toast } from "sonner";

type Tab = "live" | "archived";

export function WorkspacesSection() {
  const t_archive = "Archive";
  const { access } = useAccess();
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const qc = useQueryClient();
  const [tab, setTab] = useState<Tab>("live");
  const [creating, setCreating] = useState(false);

  const workspaces = useQuery({
    queryKey: ["workspaces", tenantId],
    queryFn: () => getWorkspaces(tenantId as string),
    enabled: !!tenantId,
    staleTime: 30_000,
  });

  const archive = useMutation({
    mutationFn: (ws: Workspace) => archiveWorkspace(ws.id, "quota"),
    onSuccess: (_data, ws) => {
      toast.success(`${ws.name} archived`, {
        description: "It no longer counts toward your plan. Restore it any time from Archived.",
      });
      void qc.invalidateQueries({ queryKey: ["workspaces", tenantId] });
    },
    onError: () => toast.error("Could not archive the workspace"),
  });

  const restore = useMutation({
    mutationFn: (ws: Workspace) => restoreWorkspace(ws.id),
    onSuccess: (_d, ws) => {
      toast.success(`${ws.name} restored`, { description: "It counts toward your plan again." });
      void qc.invalidateQueries({ queryKey: ["workspaces", tenantId] });
    },
    onError: () => toast.error("Could not restore: the plan is full."),
  });

  const { live, archived } = useMemo(() => {
    const all = workspaces.data ?? [];
    return {
      live: all.filter((w) => !w.archived_at),
      archived: all.filter((w) => Boolean(w.archived_at)),
    };
  }, [workspaces.data]);

  const rows = tab === "live" ? live : archived;
  const quota = access.quota.workspaces;

  return (
    <SettingsFrame
      title="Workspaces"
      lead={`Every workspace in ${access.organization?.name ?? "this organization"}.`}
      actions={
        access.canCreate.workspace ? (
          <Button size="sm" onClick={() => setCreating(true)} data-testid="workspaces-create">
            <Plus aria-hidden className="mr-2 h-4 w-4" />
            New workspace
          </Button>
        ) : null
      }
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <SegmentedControl<Tab>
          value={tab}
          onChange={setTab}
          ariaLabel="Workspace state"
          testId="workspaces-tab"
          options={[
            { value: "live", label: `Active · ${live.length}` },
            { value: "archived", label: `Archived · ${archived.length}` },
          ]}
        />
        <p className="text-[12.5px] tabular-nums text-muted-foreground">
          {quota.limit === 0 ? `${quota.used} workspaces` : `${quota.used} of ${quota.limit} used`}
        </p>
      </div>

      {workspaces.isLoading ? (
        <Skeleton className="h-40 w-full" />
      ) : rows.length === 0 ? (
        <p
          className="rounded-lg border bg-card p-6 text-center text-[13px] text-muted-foreground"
          data-testid={tab === "live" ? "workspaces-empty-live" : "workspaces-empty-archived"}
        >
          {tab === "live"
            ? "No active workspaces. Create one to start uploading documents."
            : "No archived workspaces. Archiving frees a plan slot without deleting data."}
        </p>
      ) : (
        <div className="divide-y rounded-lg border bg-card" data-testid="workspaces-list">
          {rows.map((w) => (
            <div key={w.id} className="flex items-center gap-3 px-3 py-2.5">
              <div className="min-w-0 flex-1">
                <Link href={`/w/${w.slug}`} className="block truncate text-[13.5px] font-medium hover:underline">
                  {w.name}
                </Link>
                <span className="font-mono text-[11.5px] text-muted-foreground">/w/{w.slug}</span>
              </div>
              <span
                className="hidden shrink-0 rounded-full bg-muted px-2 py-0.5 text-[11px] capitalize text-muted-foreground sm:block"
                title="Retrieval type (fixed at create)"
              >
                {w.rag_profile ?? "graph"}
              </span>
              {tab === "archived" &&
              access.organization?.capabilities.has("organization.settings.manage") ? (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 shrink-0 text-[12px]"
                  onClick={() => restore.mutate(w)}
                  disabled={restore.isPending}
                  data-testid={`workspace-restore-${w.slug}`}
                >
                  <RotateCcw aria-hidden className="mr-1.5 h-3.5 w-3.5" />
                  Restore
                </Button>
              ) : null}
              {tab === "live" && access.organization?.capabilities.has("organization.settings.manage") ? (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 shrink-0 text-[12px]"
                  onClick={() => archive.mutate(w)}
                  disabled={archive.isPending}
                  data-testid={`workspace-archive-${w.slug}`}
                >
                  <Archive aria-hidden className="mr-1.5 h-3.5 w-3.5" />
                  {t_archive}
                </Button>
              ) : null}
            </div>
          ))}
        </div>
      )}

      <CreateWorkspaceDialog open={creating} onOpenChange={setCreating} tenantId={tenantId} />
    </SettingsFrame>
  );
}
