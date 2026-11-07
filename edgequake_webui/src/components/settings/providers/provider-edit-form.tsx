"use client";

/**
 * Edit one deployment provider and save it on the connection.
 *
 * Environment providers start without a stored row. The first save creates one.
 * A later save updates that row. The key field is optional: blank keeps the key
 * the server already has.
 */

import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ApiRequestError, apiClient } from "@/lib/api/client";
import type { DeploymentProvider } from "@/lib/api/edgequake/provider-offer";
import { providerOfferKeys } from "@/hooks/use-provider-offer";
import { connectionLocality } from "@/components/settings/connection-locality";

function needsProject(row: DeploymentProvider): boolean {
  const shape = row.api_shape.toLowerCase();
  return shape === "vertex" || shape === "vertexai" || row.auth === "server_google_identity";
}

function urlLocked(row: DeploymentProvider): boolean {
  return (
    needsProject(row) ||
    row.auth === "aws_chain" ||
    row.auth === "server_proxy" ||
    row.api_shape === "gemini"
  );
}

export function ProviderEditForm({
  row,
  onClose,
}: {
  row: DeploymentProvider;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const projectField = needsProject(row);
  const [displayName, setDisplayName] = useState(row.display_name);
  const [baseUrl, setBaseUrl] = useState(row.base_url ?? "");
  const [project, setProject] = useState(row.google_project ?? "");
  const [apiKey, setApiKey] = useState("");
  const [busy, setBusy] = useState(false);
  const locked = urlLocked(row);
  const canSave =
    !busy &&
    displayName.trim() !== "" &&
    baseUrl.trim() !== "" &&
    (!projectField || project.trim() !== "");

  const save = async () => {
    setBusy(true);
    const place = connectionLocality(baseUrl);
    const body = {
      slug: row.slug,
      display_name: displayName.trim(),
      api_shape: row.api_shape,
      base_url: baseUrl.trim(),
      google_project: projectField ? project.trim() : undefined,
      api_key: apiKey.trim() || undefined,
      credential_region: projectField && apiKey.trim() ? "us-central1" : undefined,
      locality: place.locality,
      allow_private_network: place.allowPrivate,
    };
    try {
      if (row.connection_id) {
        await apiClient(`/connections/${row.connection_id}?scope=deployment`, {
          method: "PUT",
          omitTenant: true,
          body: JSON.stringify(body),
        });
      } else {
        try {
          await apiClient("/connections?scope=deployment", {
            method: "POST",
            omitTenant: true,
            body: JSON.stringify(body),
          });
        } catch (error) {
          if (!(error instanceof ApiRequestError) || error.status !== 409) throw error;
          const rows = await apiClient<{ id: string; slug: string; source: string }[]>("/connections");
          const existing = rows.find((item) => item.slug === row.slug && item.source !== "env");
          if (!existing) throw error;
          await apiClient(`/connections/${existing.id}?scope=deployment`, {
            method: "PUT",
            omitTenant: true,
            body: JSON.stringify(body),
          });
        }
      }
      await queryClient.invalidateQueries({ queryKey: providerOfferKeys.deployment });
      await queryClient.invalidateQueries({ queryKey: ["connections"] });
      toast.success(t("settings.providers.updated", "Provider updated"));
      onClose();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("settings.providers.saveFailed", "Save failed"));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      className="space-y-3 rounded-lg border bg-muted/20 p-3"
      data-testid={`provider-edit-${row.slug}`}
      onSubmit={(event) => {
        event.preventDefault();
        if (canSave) void save();
      }}
    >
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="space-y-1 text-xs text-muted-foreground" htmlFor={`edit-name-${row.slug}`}>
          {t("settings.providers.editName", "Name")}
          <Input
            id={`edit-name-${row.slug}`}
            data-testid={`edit-name-${row.slug}`}
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
          />
        </label>
        <label className="space-y-1 text-xs text-muted-foreground" htmlFor={`edit-url-${row.slug}`}>
          {t("settings.providers.editUrl", "Base URL")}
          <Input
            id={`edit-url-${row.slug}`}
            data-testid={`edit-url-${row.slug}`}
            value={baseUrl}
            disabled={locked}
            onChange={(event) => setBaseUrl(event.target.value)}
          />
        </label>
        {projectField ? (
          <label className="space-y-1 text-xs text-muted-foreground sm:col-span-2" htmlFor={`edit-project-${row.slug}`}>
            {t("settings.providers.editProject", "Google Cloud project")}
            <Input
              id={`edit-project-${row.slug}`}
              data-testid={`edit-project-${row.slug}`}
              value={project}
              placeholder="my-project"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setProject(event.target.value)}
            />
          </label>
        ) : null}
        <label className="space-y-1 text-xs text-muted-foreground sm:col-span-2" htmlFor={`edit-key-${row.slug}`}>
          {t("settings.providers.editKey", "API key")}
          <Input
            id={`edit-key-${row.slug}`}
            data-testid={`edit-key-${row.slug}`}
            type="password"
            value={apiKey}
            autoComplete="off"
            placeholder={t("settings.providers.editKeyHint", "Leave blank to keep the current key")}
            onChange={(event) => setApiKey(event.target.value)}
          />
        </label>
      </div>
      <div className="flex gap-2">
        <Button type="submit" size="sm" disabled={!canSave} data-testid={`edit-save-${row.slug}`}>
          {busy
            ? t("settings.providers.saving", "Saving…")
            : t("settings.providers.update", "Update provider")}
        </Button>
        <Button type="button" size="sm" variant="ghost" disabled={busy} onClick={onClose}>
          {t("common.cancel", "Cancel")}
        </Button>
      </div>
    </form>
  );
}
