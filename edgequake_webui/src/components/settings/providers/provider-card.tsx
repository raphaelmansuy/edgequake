"use client";

/**
 * SPEC-179 — one Deployment provider: who it is, whether it works, when its
 * model list was last refreshed, and what organizations may see of it.
 */

import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Check, KeyRound, TriangleAlert } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { relativeTime } from "@/lib/members/format";
import type { DeploymentProvider } from "@/lib/api/edgequake/provider-offer";
import {
  capabilitiesOf,
  offerState,
  probeTone,
  refreshSummary,
  shortModelId,
} from "@/lib/settings/provider-status";

const MODELS_SHOWN = 6;

function StatusBadges({ row }: { row: DeploymentProvider }) {
  const { t } = useTranslation();
  const tone = probeTone(row.probe);
  const state = offerState(row);
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <Badge variant="outline" className="font-normal">
        {row.origin === "env" ? t("settings.providers.origin.env", "Environment") : t("settings.providers.origin.fleet", "Custom")}
      </Badge>
      <Badge
        variant="outline"
        data-testid={`probe-${row.slug}`}
        className={cn(
          "font-normal",
          tone === "ok" && "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
          tone === "failed" && "border-destructive/40 bg-destructive/10 text-destructive",
        )}
      >
        {tone === "ok"
          ? t("settings.providers.probe.ok", "Healthy")
          : tone === "failed"
            ? t("settings.providers.probe.failed", "Check failed")
            : t("settings.providers.probe.unchecked", "Not tested")}
      </Badge>
      <Badge variant="outline" className="font-normal" data-testid={`offer-state-${row.slug}`}>
        {state === "hidden"
          ? t("settings.providers.state.hidden", "Hidden from organizations")
          : state === "offered"
            ? t("settings.providers.state.offered", "Offered")
            : t("settings.providers.state.notOffered", "Not offered")}
      </Badge>
    </div>
  );
}

function RefreshLine({ row }: { row: DeploymentProvider }) {
  const { t } = useTranslation();
  const summary = refreshSummary(row);
  if (summary.tone === "never") {
    /*
     * SPEC-184 — say what this means, because as written it contradicted the page.
     *
     * `refresh` records a *discovery run*, not whether models exist. The card printed
     * "Model list not refreshed yet" directly above a list of models, which reads as a bug. Naming
     * the action ("since this provider was added") removes the contradiction; the button that
     * performs it sits in the card header for providers that support discovery.
     */
    return (
      <p className="text-xs text-muted-foreground" data-testid={`refresh-line-${row.slug}`}>
        {t(
          "settings.providers.neverDiscovered",
          "These models come from the catalog; no discovery run since this provider was added",
        )}
      </p>
    );
  }
  const when = row.refresh ? relativeTime(row.refresh.refreshed_at) : "";
  return (
    <p className="flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground" data-testid={`refresh-line-${row.slug}`}>
      {summary.tone === "error" ? (
        <span className="inline-flex items-center gap-1 text-destructive" role="status">
          <TriangleAlert className="h-3.5 w-3.5" aria-hidden />
          {t("settings.providers.refreshFailedAt", "Last refresh failed {{when}}", { when })}
          {row.refresh?.error ? `: ${row.refresh.error}` : ""}
        </span>
      ) : (
        <span>{t("settings.providers.refreshedAt", "Refreshed {{when}}", { when })}</span>
      )}
      {summary.added > 0 ? (
        <Badge variant="secondary" className="px-1.5 py-0 text-[10px] font-normal" data-testid={`diff-added-${row.slug}`}>
          {t("settings.providers.diffAdded", "+{{count}} new", { count: summary.added })}
        </Badge>
      ) : null}
      {summary.removed > 0 ? (
        <Badge variant="secondary" className="px-1.5 py-0 text-[10px] font-normal" data-testid={`diff-removed-${row.slug}`}>
          {t("settings.providers.diffRemoved", "−{{count}} gone", { count: summary.removed })}
        </Badge>
      ) : null}
    </p>
  );
}

function ModelChips({ row }: { row: DeploymentProvider }) {
  const { t } = useTranslation();
  const [all, setAll] = useState(false);
  // Registered models are the offer; discovered ones are what the last refresh found and
  // nobody has confirmed. Showing them together muddled the two, so they are counted apart
  // and a discovered id carries its own label.
  const registered = row.models.filter((model) => model.registered !== false);
  const discovered = row.models.filter((model) => model.registered === false);
  const [openDiscovered, setOpenDiscovered] = useState(false);
  const shown = all ? registered : registered.slice(0, MODELS_SHOWN);

  if (registered.length === 0 && discovered.length === 0) {
    return (
      <p className="text-xs text-muted-foreground">
        {t("settings.providers.noModelsYet", "No registered models. Refresh to discover them.")}
      </p>
    );
  }
  return (
    <div className="space-y-1.5">
      {registered.length > 0 ? (
        <ul className="flex flex-wrap gap-1.5" aria-label={t("settings.providers.registered", "Registered models")}>
          {shown.map((model) => (
            <li key={model.id}>
              <Badge variant="outline" className="gap-1.5 font-normal" title={model.id}>
                <span className="font-mono text-[11px]">{shortModelId(model.id)}</span>
                {capabilitiesOf(model).map((capability) => (
                  <span key={capability} className="text-[10px] text-muted-foreground">
                    {t(`settings.providers.cap.${capability}`, capability)}
                  </span>
                ))}
              </Badge>
            </li>
          ))}
          {registered.length > MODELS_SHOWN ? (
            <li>
              <Button type="button" variant="ghost" size="sm" className="h-6 px-2 text-xs" onClick={() => setAll((open) => !open)}>
                {all
                  ? t("settings.providers.showLess", "Show less")
                  : t("settings.providers.showMore", "+{{count}} more", { count: registered.length - MODELS_SHOWN })}
              </Button>
            </li>
          ) : null}
        </ul>
      ) : null}
      {discovered.length > 0 ? (
        <div className="space-y-1" data-testid={`discovered-models-${row.slug}`}>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-6 px-2 text-xs text-muted-foreground"
            aria-expanded={openDiscovered}
            data-testid={`discovered-toggle-${row.slug}`}
            onClick={() => setOpenDiscovered((open) => !open)}
          >
            {openDiscovered
              ? t("settings.providers.hideDiscovered", "Hide found models")
              : t("settings.providers.showDiscovered", "{{count}} found, not registered", {
                  count: discovered.length,
                })}
          </Button>
          {openDiscovered ? (
            <>
              <ul className="flex flex-wrap gap-1.5" aria-label={t("settings.providers.discovered", "Found but not registered")}>
                {discovered.map((model) => (
                  <li key={model.id}>
                    <Badge variant="secondary" className="gap-1.5 font-normal" title={model.id}>
                      <span className="font-mono text-[11px]">{shortModelId(model.id)}</span>
                    </Badge>
                  </li>
                ))}
              </ul>
              <p className="text-xs text-muted-foreground">
                {t(
                  "settings.providers.discoveredHint",
                  "The provider reports these. Register one to make it usable for a job.",
                )}
              </p>
            </>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

/**
 * How this provider proves who it is.
 *
 * WHY not `key_configured` alone: a Vertex AI row has a stored value and no usable key (it
 * authenticates as the server's Google Cloud identity), so the card used to print "Key set" with a
 * green tick for a provider whose key is refused. The server now states the contract on the row.
 */
function CredentialLine({ row }: { row: DeploymentProvider }) {
  const { t } = useTranslation();
  if (row.accepts_saved_key === false) {
    const identity = row.auth === "aws_chain"
      ? t("settings.providers.credential.aws", "AWS credential chain")
      : row.auth === "server_proxy"
        ? t("settings.providers.credential.proxy", "Server-held credential")
        : t("settings.providers.credential.google", "Google Cloud identity");
    return (
      <span className="inline-flex items-center gap-1" data-testid={`key-${row.slug}`}>
        <KeyRound className="h-3.5 w-3.5 text-muted-foreground" aria-hidden />
        {identity}
      </span>
    );
  }
  if (row.key_configured) {
    return (
      <span className="inline-flex items-center gap-1" data-testid={`key-${row.slug}`}>
        <Check className="h-3.5 w-3.5 text-emerald-600" aria-hidden />
        {t("settings.providers.keySet", "Key set")}
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1" data-testid={`key-${row.slug}`}>
      <KeyRound className="h-3.5 w-3.5" aria-hidden />
      {row.auth === "server_google_identity"
        ? t("settings.providers.credential.google", "Google Cloud identity")
        : row.auth === "optional_api_key"
        ? t("settings.providers.keyOptional", "No key (optional)")
        : t("settings.providers.noKey", "No key")}
    </span>
  );
}

export function ProviderCard({
  row,
  busy,
  onToggleWithdraw,
  actions,
  editor,
}: {
  row: DeploymentProvider;
  busy: boolean;
  onToggleWithdraw: () => void;
  actions?: ReactNode;
  editor?: ReactNode;
}) {
  const { t } = useTranslation();
  return (
    <li
      className="space-y-3 rounded-xl border bg-card p-4 shadow-xs"
      id={`deployment-provider-${row.slug}`}
      data-testid={`deployment-provider-${row.slug}`}
    >
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0 space-y-1">
          <h3 className="truncate text-base font-medium">{row.display_name}</h3>
          <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
            {/* The registry's label, not the raw slug: `openai_chat` is an implementation term. */}
            <span data-testid={`shape-${row.slug}`}>{row.api_shape_label ?? row.api_shape}</span>
            <CredentialLine row={row} />
            {row.google_project ? (
              <span data-testid={`project-${row.slug}`}>Project {row.google_project}</span>
            ) : null}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {actions}
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={busy}
            data-testid={`withdraw-${row.slug}`}
            onClick={onToggleWithdraw}
          >
            {row.withdrawn
              ? t("settings.providers.restore", "Restore")
              : t("settings.providers.remove", "Remove")}
          </Button>
        </div>
      </div>
      <StatusBadges row={row} />
      {row.probe_error ? (
        <p className="text-xs text-destructive" role="alert" data-testid={`probe-error-${row.slug}`}>
          {row.probe_error}
        </p>
      ) : null}
      <RefreshLine row={row} />
      {editor ?? <ModelChips row={row} />}
    </li>
  );
}
