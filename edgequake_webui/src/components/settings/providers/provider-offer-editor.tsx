"use client";

/**
 * SPEC-177 — one provider editor. The parent argument selects the payload.
 * Model ticks for a provider that is off are not part of the offer save.
 */

import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { DeploymentProviders } from "@/components/settings/providers/deployment-providers";
import { Checkbox } from "@/components/ui/checkbox";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { ModelSaveBar, saveWithToast } from "@/components/settings/models/model-save-bar";
import { useDirtyGuard } from "@/hooks/use-dirty-guard";
import { useServerBackedDraft } from "@/hooks/use-server-backed-draft";
import {
  useSaveTenantOffer,
  useSaveWorkspaceOffer,
  useTenantOffer,
  useWorkspaceOffer,
} from "@/hooks/use-provider-offer";
import type { OfferProvider, ProviderOffer } from "@/lib/api/edgequake/provider-offer";import {
  draftFromProviders,
  lead,
  sameDraft,
  slugsForSave,
  toggleSlug,
  type OfferDraft,
} from "@/lib/settings/provider-offer";

const EMPTY: ProviderOffer = {
  tenant_id: "",
  offer_mode: "unconstrained",
  parent_count: 0,
  enabled_count: 0,
  can_manage: false,
  providers: [],
  legacy: [],
};

function toDraft(offer: ProviderOffer): OfferDraft {
  return draftFromProviders(
    offer.offer_mode ?? "unconstrained",
    (offer.providers ?? []).map((row) => ({ slug: row.slug, enabled: row.enabled })),
  );
}

interface GrantEditorProps {
  parent: "tenant" | "workspace";
  offer: ProviderOffer | undefined;
  pending: boolean;
  readOnly: boolean;
  loadError?: boolean;
  onRetry?: () => void;
  onSave: (body: { offer_mode: "unconstrained" | "constrained"; slugs: string[] }) => Promise<unknown>;
}

/** How many model chips a collapsed provider row shows. The rest sit behind a count toggle. */
const CHIPS_COLLAPSED = 6;

/**
 * The one-line answer to "what does this provider need?", or `null` when it needs nothing.
 *
 * The offer row used to say only how many models a provider has. A reviewer scanning it could not
 * tell a local server (nothing to configure) from a cloud provider whose identity or key was
 * missing — the two cases that decide whether a workspace can answer at all.
 */
function authHintFor(row: OfferProvider): string | null {
  switch (row.auth) {
    case "server_google_identity":
      return row.key_configured ? "Key set" : "Google Cloud identity";
    case "aws_chain":
      return "AWS credential chain";
    case "server_proxy":
      return "Server-held credential";
    case "optional_api_key":
      return null;
    case "api_key":
    case "api_key_and_endpoint":
      return row.key_configured === false ? "Key not set" : "Key set";
    default:
      return null;
  }
}

/**
 * One provider row.
 *
 * A provider with more models than `CHIPS_COLLAPSED` shows the first few and a `+N` button that
 * expands to the full list in place — the count is a control, not a dead label, so a user who
 * wants to see whether a specific model is offered does not have to leave the page.
 */
function ProviderRow({
  row,
  on,
  readOnly,
  canManage,
  onToggle,
}: {
  row: OfferProvider;
  on: boolean;
  readOnly: boolean;
  canManage: boolean;
  onToggle: () => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const models = row.models ?? [];
  const hidden = models.length - CHIPS_COLLAPSED;
  const shown = expanded ? models : models.slice(0, CHIPS_COLLAPSED);
  const authHint = authHintFor(row);

  return (
    <li
      data-testid={`provider-row-${row.slug}`}
      className="grid grid-cols-1 items-start gap-x-6 gap-y-2 px-3 py-3 lg:grid-cols-[minmax(11rem,14rem)_minmax(0,1fr)]"
    >
      <label className="flex items-start gap-3">
        <Checkbox
          className="mt-0.5"
          checked={on}
          disabled={readOnly || !canManage}
          aria-label={`Offer ${row.display_name}`}
          data-testid={`provider-toggle-${row.slug}`}
          onCheckedChange={onToggle}
        />
        <span className="min-w-0">
          <span className="block truncate font-medium">{row.display_name}</span>
          <span className="text-xs text-muted-foreground">
            {models.length} {models.length === 1 ? "model" : "models"}
            {/* SPEC-184 — what this provider needs to authenticate. Without it a reviewer cannot
                tell an entry that is silent because it is local from one that is silent because
                its identity is missing. */}
            {authHint ? <span className="text-muted-foreground/70"> · {authHint}</span> : null}
          </span>
        </span>
      </label>
      {on && models.length > 0 ? (
        <div className="flex min-w-0 flex-wrap items-center gap-1">
          {shown.map((model) => (
            <Badge key={model.id} variant="outline" className="max-w-full font-normal">
              <span className="truncate">{model.id}</span>
            </Badge>
          ))}
          {hidden > 0 ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-6 px-1.5 text-xs text-muted-foreground"
              aria-expanded={expanded}
              data-testid={`provider-more-${row.slug}`}
              onClick={() => setExpanded((current) => !current)}
            >
              {expanded ? "Show less" : `+${hidden} more`}
            </Button>
          ) : null}
        </div>
      ) : null}
    </li>
  );
}

function GrantEditor({ parent, offer, pending, readOnly, loadError, onRetry, onSave }: GrantEditorProps) {
  const { t } = useTranslation();
  const saved = offer?.providers ? offer : EMPTY;
  const { draft, setDraft, reset } = useServerBackedDraft(saved, toDraft);
  const dirty = !sameDraft(draft, toDraft(saved));
  useDirtyGuard(dirty && !loadError);
  const parentSlugs = (saved.providers ?? []).map((row) => row.slug);
  const enabled = new Set(draft.mode === "unconstrained" ? parentSlugs : draft.slugs);
  const confirm = useStateConfirm();

  const count = enabled.size;
  const sentence = loadError
    ? t("settings.providers.loadError", "Could not load providers.")
    : pending
      ? t("settings.providers.loading", "Loading providers…")
      : saved.providers.length === 0
        ? t(
            "settings.providers.empty",
            "This deployment has no providers yet. Add one under Connections.",
          )
        : lead(parent, draft.mode);

  const save = (mode: OfferDraft["mode"], slugs: string[]) =>
    saveWithToast(() => onSave({ offer_mode: mode, slugs }), "Providers saved");

  const requestSave = () => {
    if (saved.offer_mode === "unconstrained" && draft.mode === "constrained") {
      confirm.open(
        parent === "tenant"
          ? "From now on, only providers you enable here appear in workspaces."
          : "From now on, only providers you enable here appear in this workspace.",
        () => {
          void save("constrained", slugsForSave(draft));
        },
      );
      return;
    }
    void save(draft.mode, slugsForSave(draft));
  };

  return (
    <Card data-testid="provider-offer-card">
      <CardHeader className="pb-2">
        {loadError ? (
          <div role="alert" className="flex flex-wrap items-center justify-between gap-3">
            <CardDescription data-testid="provider-offer-lead" className="text-destructive">
              {sentence}
            </CardDescription>
            <Button type="button" variant="outline" size="sm" onClick={onRetry} data-testid="provider-offer-retry">
              {t("settings.providers.retry", "Retry")}
            </Button>
          </div>
        ) : (
          <CardDescription data-testid="provider-offer-lead">{sentence}</CardDescription>
        )}
      </CardHeader>
      <CardContent className="space-y-4">
        {pending ? (
          <p className="text-sm text-muted-foreground" aria-busy="true">Loading providers…</p>
        ) : (saved.providers ?? []).length === 0 ? null : (
          <>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="text-sm text-muted-foreground" data-testid="provider-offer-count">
                {count} of {saved.parent_count} providers
              </p>
              {count < saved.parent_count ? (
                <p className="text-xs text-muted-foreground">
                  Unchecked providers stay hidden from this {parent === "tenant" ? "organization" : "workspace"}.
                </p>
              ) : null}
            </div>
            <ul className="divide-y rounded-lg border">
              {(saved.providers ?? []).map((row) => (
                <ProviderRow
                  key={row.slug}
                  row={row}
                  on={enabled.has(row.slug)}
                  readOnly={readOnly}
                  canManage={saved.can_manage}
                  onToggle={() => setDraft((current) => toggleSlug(current, row.slug, parentSlugs))}
                />
              ))}
            </ul>
          </>
        )}
        {parent === "tenant" && (saved.legacy ?? []).length > 0 ? (
          <div className="rounded-lg border border-dashed px-3 py-3 text-sm" data-testid="legacy-connections">
            <p className="font-medium">This organization has a key saved earlier.</p>
            <ul className="mt-2 space-y-1 text-muted-foreground">
              {(saved.legacy ?? []).map((row) => (
                <li key={row.slug}>
                  {row.display_name}. New keys are added on Deployment.
                </li>
              ))}
            </ul>
          </div>
        ) : null}
        {saved.can_manage && !readOnly ? (
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            {draft.mode === "constrained" ? (
              <Button
                type="button"
                variant="ghost"
                className="justify-start px-0"
                onClick={() =>
                  confirm.open(
                    parent === "tenant"
                      ? "Workspaces will be able to use every provider this deployment offers, including ones added later."
                      : "This workspace will be able to use every provider the organization offers, including ones added later.",
                    () => {
                      setDraft(() => ({ mode: "unconstrained", slugs: parentSlugs }));
                      void save("unconstrained", []);
                    },
                  )
                }
              >
                {parent === "tenant"
                  ? t("settings.providers.useAllDeployment", "Use every provider this deployment offers")
                  : t("settings.providers.useAllOrganization", "Use every provider the organization offers")}
              </Button>
            ) : (
              <span />
            )}
            <ModelSaveBar dirty={dirty} saving={false} onSave={requestSave} onDiscard={reset} />
          </div>
        ) : null}
        <AlertDialog open={confirm.state.open} onOpenChange={(open) => !open && confirm.close()}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Change which providers are offered</AlertDialogTitle>
              <AlertDialogDescription>{confirm.state.message}</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction
                onClick={() => {
                  confirm.state.action();
                  confirm.close();
                }}
              >
                Confirm
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </CardContent>
    </Card>
  );
}

function useStateConfirm() {
  const [state, setState] = useState<{ open: boolean; message: string; action: () => void }>({
    open: false,
    message: "",
    action: () => undefined,
  });
  return {
    state,
    open: (message: string, action: () => void) => setState({ open: true, message, action }),
    close: () => setState((current) => ({ ...current, open: false })),
  };
}

export function ProviderOfferEditor({
  parent,
  tenantId,
  workspaceId,
  readOnly = false,
}: {
  parent: "deployment" | "tenant" | "workspace";
  tenantId?: string | null;
  workspaceId?: string | null;
  readOnly?: boolean;
}) {
  if (parent === "deployment") return <DeploymentProviders />;
  if (parent === "tenant") return <TenantProviders tenantId={tenantId ?? null} readOnly={readOnly} />;
  return <WorkspaceProviders workspaceId={workspaceId ?? null} readOnly={readOnly} />;
}

function TenantProviders({ tenantId, readOnly }: { tenantId: string | null; readOnly: boolean }) {
  const query = useTenantOffer(tenantId);
  const save = useSaveTenantOffer(tenantId ?? "");
  if (!tenantId) return null;
  return (
    <GrantEditor
      parent="tenant"
      offer={query.data}
      pending={query.isPending}
      loadError={query.isError}
      onRetry={() => void query.refetch()}
      readOnly={readOnly}
      onSave={(body) => save.mutateAsync(body)}
    />
  );
}

function WorkspaceProviders({ workspaceId, readOnly }: { workspaceId: string | null; readOnly: boolean }) {
  const query = useWorkspaceOffer(workspaceId);
  const save = useSaveWorkspaceOffer(workspaceId ?? "");
  if (!workspaceId) return null;
  return (
    <GrantEditor
      parent="workspace"
      offer={query.data}
      pending={query.isPending}
      loadError={query.isError}
      onRetry={() => void query.refetch()}
      readOnly={readOnly || query.data?.can_manage === false}
      onSave={(body) => save.mutateAsync(body)}
    />
  );
}
