"use client";

/**
 * SPEC-181 W-04 — create an organization.
 *
 * Three decisions: name, owner, default models. The owner field exists because a platform admin
 * usually creates an organization *for someone else*, and the most common failure is "created
 * for me, now I own the customer's data". Naming the owner at creation removes a second
 * transaction (transfer ownership) and an audit ambiguity.
 *
 * The self-serve variant asks only for the name: the owner and the models are implied.
 */

import { ScopeDialog } from "@/components/admin/scope-dialog";
import { InheritLine } from "@/components/admin/primitives";
import { CreateOrganizationConfirmation } from "@/components/admin/created-confirmation";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useAccess } from "@/lib/access/use-access";
import {
  applyCreatedScope,
  classifyCreateFailure,
  settleAfterCreate,
  useCreateOrganization,
} from "@/hooks/use-create-scope";
import { useServerModelDefaults } from "@/hooks/use-server-model-defaults";
import { slugifyName } from "@/lib/onboarding/slug";
import { cn } from "@/lib/utils";
import { AlertCircle } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useMemo, useRef, useState } from "react";

type OwnerKind = "self" | "email";

export interface CreateOrganizationDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated?: (tenantId: string, workspaceSlug: string | null) => void;
}

export function CreateOrganizationDialog({
  open,
  onOpenChange,
  onCreated,
}: CreateOrganizationDialogProps) {
  const { access } = useAccess();
  const server = useServerModelDefaults();
  const create = useCreateOrganization();
  const qc = useQueryClient();
  const router = useRouter();

  const [name, setName] = useState("");
  const [ownerKind, setOwnerKind] = useState<OwnerKind>("self");
  const [ownerEmail, setOwnerEmail] = useState("");
  const [useServerDefaults, setUseServerDefaults] = useState(true);
  const [plan, setPlan] = useState("free");
  const [error, setError] = useState<{ kind: string; message: string } | null>(null);
  const [created, setCreated] = useState<{
    tenant: import("@/types").Tenant;
    workspace: import("@/types").Workspace | null;
    tenantName: string;
    tenantSlug: string;
    ownerLabel: string;
    workspaceSlug: string | null;
    plan: string;
    limit: number;
    seats: number;
  } | null>(null);

  const slug = useMemo(() => slugifyName(name), [name]);
  const trimmed = name.trim();
  const emailValid = /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(ownerEmail.trim());
  const canFinish =
    trimmed.length > 0 &&
    (ownerKind === "self" || emailValid) &&
    !create.isPending;

  const isPlatform = access.platform.admin;
  const blockedReason = !trimmed
    ? "Give the organization a name."
    : ownerKind === "email" && !emailValid
      ? "Enter a valid email address for the owner."
      : null;

  /** Held until the dialog closes, so a parent re-render cannot unmount the confirmation. */
  const pendingScope = useRef<{ tenant: import("@/types").Tenant; workspace: import("@/types").Workspace | null } | null>(null);

  const reset = () => {
    setName("");
    setOwnerKind("self");
    setOwnerEmail("");
    setUseServerDefaults(true);
    setPlan("free");
    setError(null);
    setCreated(null);
  };

  const close = (next: boolean) => {
    if (!next) {
      const pending = pendingScope.current;
      pendingScope.current = null;
      if (pending) {
        applyCreatedScope(pending.tenant, pending.workspace);
        settleAfterCreate(qc, pending.tenant.id, pending.workspace?.id ?? null);
        onCreated?.(pending.tenant.id, pending.workspace?.slug ?? null);
      }
      reset();
    }
    onOpenChange(next);
  };

  const finish = () => {
    if (!canFinish) return;
    setError(null);
    create.mutate(
      {
        name: trimmed,
        slug: slug || undefined,
        plan: isPlatform ? plan : undefined,
        owner:
          ownerKind === "email"
            ? { kind: "email", email: ownerEmail.trim() }
            : { kind: "self_principal" },
        // "Server defaults" means send nothing, so the tenant keeps the deployment ladder.
        ...(useServerDefaults
          ? {}
          : {
              default_llm_provider: server.defaultLlmProvider,
              default_llm_model: server.defaultLlmModel,
              default_embedding_provider: server.defaultEmbeddingProvider,
              default_embedding_model: server.defaultEmbeddingModel,
            }),
      },
      {
        onSuccess: ({ tenant, workspace }) => {
          setCreated({
            tenant,
            workspace,
            tenantName: tenant.name,
            tenantSlug: tenant.slug ?? slug,
            ownerLabel: ownerKind === "email" ? ownerEmail.trim() : "You",
            workspaceSlug: workspace?.slug ?? null,
            plan: tenant.plan ?? plan,
            limit: tenant.max_workspaces ?? 10,
            seats: tenant.max_users ?? 3,
          });
          pendingScope.current = { tenant, workspace };
        },
        onError: (e) => setError(classifyCreateFailure(e)),
      },
    );
  };

  // The confirmation replaces the form in the same dialog; a toast would be a dead end.
  if (created) {
    return (
      <ScopeDialog
        open={open}
        onOpenChange={close}
        title="Organization created"
        canFinish
        finishLabel="Done"
        hideCancel
        isDirty={false}
        onFinish={() => close(false)}
        testId="create-organization-dialog"
      >
        <CreateOrganizationConfirmation
          {...created}
          onInvite={() => {
            close(false);
            router.push("/members");
          }}
          onUpload={() => {
            close(false);
            router.push("/documents");
          }}
        />
      </ScopeDialog>
    );
  }

  return (
    <ScopeDialog
      open={open}
      onOpenChange={close}
      title="Create an organization"
      lead="An organization holds workspaces, people and one plan."
      canFinish={canFinish}
      blockedReason={blockedReason}
      busy={create.isPending}
      finishLabel="Create organization"
      isDirty={trimmed.length > 0 || ownerKind === "email"}
      onFinish={finish}
      testId="create-organization-dialog"
    >
      <div className="space-y-5">
        <div className="space-y-1.5">
          <Label htmlFor="org-name" className="text-[13px] font-medium">
            Name
          </Label>
          <Input
            id="org-name"
            data-autofocus
            data-wizard-enter
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Globex Holdings"
            autoComplete="off"
            maxLength={120}
            aria-invalid={error?.kind === "slug"}
            data-testid="create-organization-name"
          />
          {slug ? <p className="font-mono text-[11.5px] text-muted-foreground">{slug}</p> : null}
        </div>

        <div className="space-y-1.5">
          <Label className="text-[13px] font-medium">Owner</Label>
          <div role="radiogroup" aria-label="Owner" className="space-y-2">
            <OwnerChoice
              checked={ownerKind === "self"}
              onSelect={() => setOwnerKind("self")}
              title="Me"
              body="You will be the organization's owner."
              testId="create-organization-owner-self"
            />
            <OwnerChoice
              checked={ownerKind === "email"}
              onSelect={() => setOwnerKind("email")}
              title="Someone else"
              body="They receive an invitation. You stay owner until they accept."
              testId="create-organization-owner-email"
            />
          </div>
          {ownerKind === "email" ? (
            <Input
              value={ownerEmail}
              onChange={(e) => setOwnerEmail(e.target.value)}
              placeholder="owner@company.com"
              type="email"
              autoComplete="off"
              aria-label="Owner email"
              aria-invalid={ownerEmail.length > 0 && !emailValid}
              data-testid="create-organization-owner-email-field"
            />
          ) : null}
        </div>

        <div className="space-y-1.5">
          <Label className="text-[13px] font-medium">Default models for new workspaces</Label>
          <div className="rounded-lg border bg-muted/30 p-3">
            <label className="flex items-start gap-2">
              <input
                type="radio"
                name="models"
                checked={useServerDefaults}
                onChange={() => setUseServerDefaults(true)}
                className="mt-1"
                data-testid="create-organization-models-server"
              />
              <span className="min-w-0">
                <span className="block text-[12.5px] font-medium text-foreground">Server defaults</span>
                <span className="mt-0.5 flex flex-wrap gap-x-3 gap-y-1">
                  <InheritLine
                    layer={null}
                    value={
                      server.isLoading
                        ? "loading…"
                        : `${server.defaultLlmProvider ?? "?"}/${server.defaultLlmModel ?? "?"}`
                    }
                  />
                </span>
              </span>
            </label>
          </div>
        </div>

        {/* Platform-only: the plan picks the quota. An owner cannot raise their own limit. */}
        {isPlatform ? (
          <div className="space-y-1.5">
            <Label htmlFor="org-plan" className="text-[13px] font-medium">
              Plan
            </Label>
            <Select value={plan} onValueChange={setPlan}>
              <SelectTrigger id="org-plan" size="sm" className="w-full" data-testid="create-organization-plan">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="free">Free — 10 workspaces, 3 people</SelectItem>
                <SelectItem value="basic">Basic — 100 workspaces, 10 people</SelectItem>
                <SelectItem value="pro">Pro — 500 workspaces, 50 people</SelectItem>
                <SelectItem value="enterprise">Enterprise — 500 workspaces, unlimited people</SelectItem>
              </SelectContent>
            </Select>
          </div>
        ) : null}

        {error ? (
          <div
            role="alert"
            data-testid="create-organization-error"
            className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-[12.5px]"
          >
            <AlertCircle aria-hidden className="mt-0.5 h-3.5 w-3.5 shrink-0 text-destructive" />
            <span>
              {error.kind === "slug"
                ? "That address is already taken. Change the name and try again."
                : "Could not create the organization. Your input is kept, so you can retry."}
            </span>
          </div>
        ) : null}
      </div>
    </ScopeDialog>
  );
}

function OwnerChoice({
  checked,
  onSelect,
  title,
  body,
  testId,
}: {
  checked: boolean;
  onSelect: () => void;
  title: string;
  body: string;
  testId: string;
}) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={checked}
      onClick={onSelect}
      data-testid={testId}
      className={cn(
        "flex w-full items-start gap-3 rounded-lg border p-3 text-left transition-colors",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        checked ? "border-primary bg-primary/5 ring-1 ring-primary" : "border-border hover:bg-muted/40",
      )}
    >
      <span
        aria-hidden
        className={cn(
          "mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-full border",
          checked ? "border-primary" : "border-input",
        )}
      >
        {checked ? <span className="h-2 w-2 rounded-full bg-primary" /> : null}
      </span>
      <span className="min-w-0">
        <span className="block text-[13.5px] font-medium text-foreground">{title}</span>
        <span className="mt-0.5 block text-[12.5px] leading-relaxed text-muted-foreground">{body}</span>
      </span>
    </button>
  );
}
