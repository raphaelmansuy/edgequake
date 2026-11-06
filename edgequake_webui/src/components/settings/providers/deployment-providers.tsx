"use client";

/**
 * SPEC-179 — Deployment › Providers. Every provider an organization can turn on:
 * environment keys and custom connections. Withdrawing one that is still in use
 * asks first, and says how many assignments it breaks.
 */

import { useState } from "react";
import Link from "next/link";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
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
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { FleetModels } from "@/components/settings/providers/fleet-models";
import { ProviderCard } from "@/components/settings/providers/provider-card";
import { providerOfferKeys, useDeploymentProviders, useWithdrawProvider } from "@/hooks/use-provider-offer";
import { apiClient } from "@/lib/api/client";
import { refreshDynamicModels, useAvailableProviders } from "@/hooks/use-providers";
import {
  needsConfirm,
  refreshProviderModels,
  type DeploymentProvider,
} from "@/lib/api/edgequake/provider-offer";
import { showProviderModelRefresh } from "@/lib/settings/fleet-model-refresh";
import { sectionHref } from "@/lib/settings/registry";

interface PendingWithdraw {
  slug: string;
  name: string;
  message: string;
}

function EnvRefresh({ row }: { row: DeploymentProvider }) {
  const { t } = useTranslation();
  const providers = useAvailableProviders();
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const refreshable =
    providers.data?.llm_providers.some((item) => item.id === row.slug && item.refreshable === true) ?? false;
  if (!showProviderModelRefresh({ refreshable, origin: "env", probe: row.probe })) return null;

  const run = async () => {
    setBusy(true);
    try {
      const result = await refreshProviderModels(row.slug);
      await refreshDynamicModels(queryClient);
      void queryClient.invalidateQueries({ queryKey: providerOfferKeys.deployment });
      toast.success(
        result.added.length + result.removed.length > 0
          ? t("settings.providers.refreshedDiff", "Models refreshed: +{{added}} new, −{{removed}} gone", {
              added: result.added.length,
              removed: result.removed.length,
            })
          : t("settings.providers.refreshed", "Models refreshed"),
      );
    } catch {
      toast.error(t("settings.providers.refreshFailed", "Could not refresh models."));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Button
      type="button"
      size="sm"
      variant="outline"
      disabled={busy}
      data-testid={`env-refresh-${row.slug}`}
      onClick={() => void run()}
    >
      {busy ? t("settings.providers.refreshing", "Refreshing…") : t("settings.providers.refresh", "Refresh models")}
    </Button>
  );
}

function FleetTest({ row }: { row: DeploymentProvider }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  if (row.origin !== "fleet" || !row.connection_id || row.probe === "ok") return null;

  const run = async () => {
    setBusy(true);
    try {
      const result = await apiClient<{ ok: boolean; message?: string }>(
        `/connections/${row.connection_id}/test?scope=deployment`,
        { method: "POST", omitTenant: true },
      );
      await queryClient.invalidateQueries({ queryKey: providerOfferKeys.deployment });
      await queryClient.invalidateQueries({ queryKey: ["tenant-provider-offer"] });
      if (result.ok) {
        toast.success(
          t("settings.providers.testOffered", "{{name}} is offered to organizations.", {
            name: row.display_name,
          }),
        );
      } else {
        toast.error(result.message || t("settings.providers.testFailed", "The connection test failed."));
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("settings.providers.testFailed", "The connection test failed."));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Button
      type="button"
      size="sm"
      disabled={busy}
      data-testid={`test-provider-${row.slug}`}
      onClick={() => void run()}
    >
      {busy
        ? t("settings.providers.testing", "Testing…")
        : t("settings.providers.testConnection", "Test connection")}
    </Button>
  );
}

function ListSkeleton() {
  return (
    <div className="space-y-3" aria-busy="true" data-testid="deployment-providers-loading">
      {[0, 1].map((key) => (
        <Skeleton key={key} className="h-28 w-full rounded-xl" />
      ))}
    </div>
  );
}

export function DeploymentProviders() {
  const { t } = useTranslation();
  const query = useDeploymentProviders();
  const withdraw = useWithdrawProvider();
  const [pending, setPending] = useState<PendingWithdraw | null>(null);
  const rows = query.data?.providers ?? [];
  const addHref = sectionHref("connections");

  const toggle = async (row: DeploymentProvider) => {
    try {
      await withdraw.mutateAsync({ slug: row.slug, withdrawn: !row.withdrawn });
    } catch (error) {
      if (needsConfirm(error)) {
        setPending({ slug: row.slug, name: row.display_name, message: error.message });
        return;
      }
      toast.error(error instanceof Error ? error.message : t("settings.providers.saveFailed", "Save failed"));
    }
  };

  const confirmWithdraw = async () => {
    if (!pending) return;
    try {
      await withdraw.mutateAsync({ slug: pending.slug, withdrawn: true, confirm: true });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("settings.providers.saveFailed", "Save failed"));
    } finally {
      setPending(null);
    }
  };

  return (
    <div className="space-y-4" data-testid="deployment-providers-card">
      <div className="space-y-4">
        <div className="flex justify-end">
          <Button type="button" size="sm" asChild>
            <Link href={addHref} data-testid="add-provider-link">
              {t("settings.providers.add_provider", "Add a provider")}
            </Link>
          </Button>
        </div>
        {query.isPending ? (
          <ListSkeleton />
        ) : query.isError ? (
          <div className="flex flex-wrap items-center gap-3" role="alert">
            <p className="text-sm text-destructive">{t("settings.providers.loadError", "Could not load providers.")}</p>
            <Button type="button" size="sm" variant="outline" onClick={() => void query.refetch()} data-testid="deployment-providers-retry">
              {t("settings.providers.retry", "Retry")}
            </Button>
          </div>
        ) : rows.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            {t("settings.providers.deploymentEmpty", "This deployment has no providers yet.")}{" "}
            <Link href={addHref} className="underline">
              {t("settings.providers.addUnderConnections", "Add one under Connections.")}
            </Link>
          </p>
        ) : (
          <ul className="space-y-3">
            {rows.map((row) => (
              <ProviderCard
                key={row.slug}
                row={row}
                busy={withdraw.isPending}
                onToggleWithdraw={() => void toggle(row)}
                actions={row.origin === "env" ? <EnvRefresh row={row} /> : <FleetTest row={row} />}
                editor={
                  row.origin === "fleet" ? (
                    <FleetModels
                      slug={row.slug}
                      models={row.models}
                      enabled={row.probe === "ok"}
                      canTest={row.probe !== "ok" && Boolean(row.connection_id)}
                    />
                  ) : undefined
                }
              />
            ))}
          </ul>
        )}
      </div>
      <AlertDialog open={pending !== null} onOpenChange={(open) => !open && setPending(null)}>
        <AlertDialogContent data-testid="withdraw-confirm">
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("settings.providers.removeTitle", "Remove {{name}}?", { name: pending?.name ?? "" })}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {pending?.message}{" "}
              {t(
                "settings.providers.withdrawBody",
                "Those assignments keep their choice and fail with “not eligible” until an admin picks another model.",
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common.cancel", "Cancel")}</AlertDialogCancel>
            <AlertDialogAction onClick={() => void confirmWithdraw()}>
              {t("settings.providers.remove", "Remove")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
