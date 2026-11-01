"use client";

/**
 * SPEC-181 W-07 — Deployment / Organizations.
 *
 * The pre-181 product had no organization list at all: quota management lived in a small card
 * inside Observability, with no way to see every tenant on the deployment, create one, or
 * delete one. This is that page.
 */

import { CreateOrganizationDialog } from "@/components/admin/create-organization-dialog";
import { MeterRow } from "@/components/admin/primitives";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useAccess } from "@/lib/access/use-access";
import { getTenants } from "@/lib/api/edgequake";
import type { Tenant } from "@/types";
import { useQuery } from "@tanstack/react-query";
import { Plus, Search } from "lucide-react";
import Link from "next/link";
import { useMemo, useState } from "react";

export function OrganizationsSection() {
  const { access } = useAccess();
  const [search, setSearch] = useState("");
  const [creating, setCreating] = useState(false);

  const tenants = useQuery({
    queryKey: ["tenants"],
    queryFn: getTenants,
    staleTime: 30_000,
    enabled: access.platform.admin,
  });

  const rows = useMemo(() => {
    const all = tenants.data ?? [];
    const q = search.trim().toLowerCase();
    return q
      ? all.filter((t) => t.name.toLowerCase().includes(q) || (t.slug ?? "").toLowerCase().includes(q))
      : all;
  }, [tenants.data, search]);

  return (
    <SettingsFrame
      title="Organizations"
      lead="Every organization on this deployment."
      actions={
        access.canCreate.organization ? (
          <Button size="sm" onClick={() => setCreating(true)} data-testid="organizations-create">
            <Plus aria-hidden className="mr-2 h-4 w-4" />
            New
          </Button>
        ) : null
      }
    >
      <div className="relative max-w-sm">
        <Search aria-hidden className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
        <Input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search organizations"
          className="h-8 pl-8 text-[13px]"
          data-testid="organizations-search"
        />
      </div>

      {tenants.isLoading ? (
        <Skeleton className="h-40 w-full" />
      ) : rows.length === 0 ? (
        <p className="rounded-lg border bg-card p-6 text-center text-[13px] text-muted-foreground">
          {search ? "No organization matches that search." : "No organizations on this deployment yet."}
        </p>
      ) : (
        <div className="overflow-x-auto rounded-lg border bg-card" data-testid="organizations-table">
          <table className="w-full min-w-[46rem] text-[13px]">
            <thead className="bg-muted/30">
              <tr>
                {["Organization", "Plan", "Workspaces", "People", "Status"].map((h) => (
                  <th
                    key={h}
                    scope="col"
                    className="px-3 py-2 text-left text-[11px] font-medium uppercase tracking-[0.06em] text-muted-foreground"
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((t) => (
                <OrganizationRow key={t.id} tenant={t} />
              ))}
            </tbody>
          </table>
        </div>
      )}

      <CreateOrganizationDialog open={creating} onOpenChange={setCreating} />
    </SettingsFrame>
  );
}

function OrganizationRow({ tenant }: { tenant: Tenant }) {
  const archived = Boolean(tenant.archived_at);
  const used = tenant.workspaces_used ?? 0;
  const limit = tenant.max_workspaces ?? 0;
  const seats = tenant.members_used ?? 0;
  const seatLimit = tenant.max_users ?? 0;
  // An organization over its people limit was lowered after the fact; show it in amber so the
  // operator can see the overage rather than discovering it at the next invite (EC-181-18).
  const overSeats = seatLimit > 0 && seats > seatLimit;

  return (
    <tr className="border-t" data-testid={`organization-row-${tenant.slug ?? tenant.id}`}>
      <th scope="row" className="px-3 py-2 text-left font-normal">
        <Link href="/settings/deployment/organizations" className="font-medium hover:underline">
          {tenant.name}
        </Link>
        <span className="ml-2 font-mono text-[11.5px] text-muted-foreground">{tenant.slug}</span>
      </th>
      <td className="px-3 py-2 capitalize text-muted-foreground">{tenant.plan ?? "free"}</td>
      <td className="w-40 px-3 py-2">
        <MeterRow label="" used={used} limit={limit} className="space-y-1" />
      </td>
      <td className="w-36 px-3 py-2">
        <span className={overSeats ? "tabular-nums text-amber-600 dark:text-amber-400" : "tabular-nums"}>
          {seatLimit === 0 ? `${seats} · unlimited` : `${seats} / ${seatLimit}`}
        </span>
      </td>
      <td className="px-3 py-2">
        <span
          className={
            archived
              ? "rounded-full bg-muted px-2 py-0.5 text-[11px] font-medium text-muted-foreground"
              : "rounded-full bg-success/10 px-2 py-0.5 text-[11px] font-medium text-success"
          }
        >
          {archived ? "Archived" : "Active"}
        </span>
      </td>
    </tr>
  );
}
