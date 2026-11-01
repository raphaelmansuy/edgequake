"use client";

/**
 * SPEC-181 W-09 — the grant editor.
 *
 * One person, many grants. The pre-181 members table rendered a single `role` cell because
 * `primary_membership` collapsed a person's grants to one, so "Admin of Legal, Viewer of Finance,
 * Member of the organization" was invisible and uneditable
 * ([F-181-05](../../../../specs/181-admin/04-findings.md)).
 *
 * The role select is filtered by the actor's rank *in that scope*, so an Admin never sees a
 * role that would be refused, and the last-owner guard is stated rather than discovered.
 */

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ShieldCheck, Trash2 } from "lucide-react";
import { cn } from "@/lib/utils";

export interface GrantRowModel {
  membership_id: string;
  user_id: string;
  scope: "organization" | "workspace";
  workspace_id?: string | null;
  workspace_name?: string | null;
  role: string;
  level: number;
  sso_managed?: boolean;
}

const ROLE_LABEL: Record<string, string> = {
  owner: "Owner",
  admin: "Admin",
  member: "Member",
  readonly: "Viewer",
};

/** Roles at or below `maxLevel`, highest first. */
export function assignableRoles(maxLevel: number): { value: string; label: string }[] {
  const order: [string, number][] = [
    ["owner", 4],
    ["admin", 3],
    ["member", 2],
    ["readonly", 1],
  ];
  return order
    .filter(([, level]) => level <= maxLevel)
    .map(([value]) => ({ value, label: ROLE_LABEL[value] ?? value }));
}

export function GrantEditor({
  grants,
  workspaces,
  /** The viewer's highest level in the organization; caps what may be assigned. */
  maxLevel,
  isLastOwner,
  busy,
  onChangeRole,
  onRemove,
  onAdd,
  testId = "grant-editor",
}: {
  grants: readonly GrantRowModel[];
  /** Workspaces the viewer may add a grant on. */
  workspaces: readonly { id: string; name: string }[];
  maxLevel: number;
  /** The person holds the only organization Owner grant. */
  isLastOwner: boolean;
  busy?: boolean;
  onChangeRole: (grant: GrantRowModel, role: string) => void;
  onRemove: (grant: GrantRowModel) => void;
  onAdd: (workspaceId: string | null, role: string) => void;
  testId?: string;
}) {
  const orgGrant = grants.find((g) => g.scope === "organization");
  const wsGrants = grants.filter((g) => g.scope === "workspace");
  const takenWorkspaces = new Set(wsGrants.map((g) => g.workspace_id));
  const addable = workspaces.filter((w) => !takenWorkspaces.has(w.id));
  const roles = assignableRoles(maxLevel);

  return (
    <div className="space-y-4" data-testid={testId}>
      <Section label="Organization">
        {orgGrant ? (
          <GrantLine
            grant={orgGrant}
            roles={roles}
            busy={busy}
            locked={isLastOwner}
            lockedReason="The organization needs a second owner before this one can change."
            onChangeRole={onChangeRole}
            onRemove={onRemove}
          />
        ) : (
          <p className="px-1 py-2 text-[12.5px] text-muted-foreground">
            No organization grant: this person reaches only the workspaces below.
          </p>
        )}
      </Section>

      <Section label={`Workspaces · ${wsGrants.length}`}>
        {wsGrants.length > 0 ? (
          wsGrants.map((g) => (
            <GrantLine
              key={g.membership_id}
              grant={g}
              roles={roles}
              busy={busy}
              locked={false}
              onChangeRole={onChangeRole}
              onRemove={onRemove}
            />
          ))
        ) : (
          <p className="px-1 py-2 text-[12.5px] text-muted-foreground">
            No workspace grants. An organization grant already covers every workspace.
          </p>
        )}

        {addable.length > 0 ? (
          <div className="flex flex-wrap items-center gap-2 px-1 pt-2">
            <Select
              value=""
              onValueChange={(ws) => onAdd(ws, roles[roles.length - 1]?.value ?? "readonly")}
            >
              <SelectTrigger size="sm" className="w-[11rem]" data-testid={`${testId}-add`}>
                <SelectValue placeholder="+ Add a workspace grant" />
              </SelectTrigger>
              <SelectContent>
                {addable.map((w) => (
                  <SelectItem key={w.id} value={w.id}>
                    {w.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        ) : null}
      </Section>

      <p className="text-[11.5px] leading-relaxed text-muted-foreground">
        What someone can see inside a workspace is set by document labels, under that
        workspace&apos;s Access settings.
      </p>
    </div>
  );
}

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <section className="space-y-1">
      <h4 className="text-[11px] font-semibold uppercase tracking-[0.06em] text-muted-foreground">
        {label}
      </h4>
      <div className="divide-y rounded-lg border bg-card">{children}</div>
    </section>
  );
}

function GrantLine({
  grant,
  roles,
  busy,
  locked,
  lockedReason,
  onChangeRole,
  onRemove,
}: {
  grant: GrantRowModel;
  roles: { value: string; label: string }[];
  busy?: boolean;
  locked: boolean;
  lockedReason?: string;
  onChangeRole: (grant: GrantRowModel, role: string) => void;
  onRemove: (grant: GrantRowModel) => void;
}) {
  const scopeName =
    grant.scope === "organization" ? "Whole organization" : (grant.workspace_name ?? "Workspace");

  return (
    <div className="flex flex-wrap items-center gap-2 px-3 py-2" data-testid={`grant-${grant.membership_id}`}>
      <div className="min-w-0 flex-1">
        <p className="truncate text-[13px] font-medium text-foreground">{scopeName}</p>
        {grant.sso_managed ? (
          <p className="mt-0.5 flex items-center gap-1 text-[11px] text-muted-foreground">
            <ShieldCheck aria-hidden className="h-3 w-3" />
            Managed by your identity provider · editing pins it
          </p>
        ) : null}
        {locked && lockedReason ? (
          <p className="mt-0.5 text-[11px] text-amber-600 dark:text-amber-400">{lockedReason}</p>
        ) : null}
      </div>

      <Select
        value={grant.role}
        onValueChange={(r) => onChangeRole(grant, r)}
        disabled={busy || locked}
      >
        <SelectTrigger
          size="sm"
          className="w-[6.5rem]"
          aria-label={`Role for ${scopeName}`}
          data-testid={`grant-role-${grant.membership_id}`}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {roles.map((r) => (
            <SelectItem key={r.value} value={r.value}>
              {r.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <Button
        variant="ghost"
        size="icon"
        className={cn("h-7 w-7 shrink-0 text-muted-foreground hover:text-destructive")}
        disabled={busy || locked}
        onClick={() => onRemove(grant)}
        aria-label={`Remove the ${scopeName} grant`}
        data-testid={`grant-remove-${grant.membership_id}`}
      >
        <Trash2 aria-hidden className="h-3.5 w-3.5" />
      </Button>
    </div>
  );
}
