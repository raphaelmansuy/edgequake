"use client";

/**
 * SPEC-179 — the models an admin has confirmed for one deployment provider.
 * Discovered ids stay unregistered until ticked (LAW-179-6). Embedding models
 * carry their vector dimension; the server refuses one without it.
 */

import { useMemo, useState } from "react";
import Link from "next/link";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import {
  ModelSaveBar,
  saveWithToast,
} from "@/components/settings/models/model-save-bar";
import { useServerBackedDraft } from "@/hooks/use-server-backed-draft";
import {
  refreshDynamicModels,
  useAvailableProviders,
} from "@/hooks/use-providers";
import { useSaveConnectionModels } from "@/hooks/use-provider-offer";
import { apiClient } from "@/lib/api/client";
import type { OfferModel } from "@/lib/api/edgequake/provider-offer";
import type { LlmModelItem, LlmModelsResponse } from "@/lib/api/models";
import { sectionHref } from "@/lib/settings/registry";
import { showFleetModelRefresh } from "@/lib/settings/fleet-model-refresh";
import {
  draftFromOffer,
  embeddingsMissingDimension,
  payloadFromDraft,
  sameDraft,
  shortModelId,
  type ModelDraft,
} from "@/lib/settings/provider-status";

function serialise(models: OfferModel[]): string {
  return JSON.stringify(
    models.map((model) => [
      model.id,
      model.usages,
      model.embedding_dimension ?? null,
    ]),
  );
}

function CapabilityChips({ entry }: { entry: ModelDraft }) {
  const { t } = useTranslation();
  return (
    <span className="flex gap-1">
      <Badge
        variant="secondary"
        className="px-1.5 py-0 text-[10px] font-normal"
      >
        {entry.kind === "embedding"
          ? t("settings.providers.cap.embedding", "embedding")
          : t("settings.providers.cap.chat", "chat")}
      </Badge>
      {entry.vision ? (
        <Badge
          variant="secondary"
          className="px-1.5 py-0 text-[10px] font-normal"
        >
          {t("settings.providers.cap.vision", "vision")}
        </Badge>
      ) : null}
    </span>
  );
}

export function FleetModels({
  slug,
  models: saved,
  enabled,
  canTest = false,
}: {
  slug: string;
  models: OfferModel[];
  enabled: boolean;
  /** The card header has a Test connection button. */
  canTest?: boolean;
}) {
  const { t } = useTranslation();
  const save = useSaveConnectionModels(slug);
  const providers = useAvailableProviders();
  const queryClient = useQueryClient();
  const refreshable =
    providers.data?.llm_providers.some(
      (row) => row.id === slug && row.refreshable === true,
    ) ?? false;
  const canRefresh = showFleetModelRefresh(refreshable, enabled);
  const savedKey = useMemo(() => serialise(saved), [saved]);
  const { draft, setDraft, reset } = useServerBackedDraft(savedKey, () =>
    draftFromOffer(saved),
  );
  const savedDraft = useMemo(() => draftFromOffer(saved), [saved]);
  const dirty = !sameDraft(draft, savedDraft);
  const missing = embeddingsMissingDimension(draft);
  const [newId, setNewId] = useState("");
  const [newKind, setNewKind] = useState<"chat" | "embedding">("chat");
  const [newDimension, setNewDimension] = useState("");
  const [discovered, setDiscovered] = useState<LlmModelItem[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const fieldId = `${slug}-model`;

  const refresh = async () => {
    setRefreshing(true);
    setRefreshError(null);
    try {
      await apiClient(
        `/deployment/providers/${encodeURIComponent(slug)}/refresh`,
        { method: "POST" },
      );
      await refreshDynamicModels(queryClient);
      const response = await apiClient<LlmModelsResponse>(
        `/models/llm?provider=${encodeURIComponent(slug)}`,
      );
      setDiscovered(response.models ?? []);
    } catch {
      setRefreshError(
        t("settings.providers.refreshFailed", "Could not refresh models."),
      );
    } finally {
      setRefreshing(false);
    }
  };

  const toggleDiscovered = (item: LlmModelItem, checked: boolean) =>
    setDraft((current) =>
      checked
        ? current.some((entry) => entry.id === item.name)
          ? current
          : [
              ...current,
              {
                id: item.name,
                kind: "chat",
                vision: item.capabilities.supports_vision === true,
              },
            ]
        : current.filter((entry) => entry.id !== item.name),
    );

  const addEntry = () => {
    const id = newId.trim();
    if (!id || draft.some((entry) => entry.id === id)) return;
    const dimension = Number.parseInt(newDimension, 10);
    setDraft((current) => [
      ...current,
      {
        id,
        kind: newKind,
        vision: false,
        dimension:
          newKind === "embedding" && dimension > 0 ? dimension : undefined,
      },
    ]);
    setNewId("");
    setNewDimension("");
  };

  const setDimension = (id: string, value: string) => {
    const dimension = Number.parseInt(value, 10);
    setDraft((current) =>
      current.map((entry) =>
        entry.id === id
          ? { ...entry, dimension: dimension > 0 ? dimension : undefined }
          : entry,
      ),
    );
  };

  const saveModels = () => {
    if (missing.length > 0) {
      toast.error(
        t(
          "settings.providers.dimensionRequired",
          "An embedding model needs its dimension.",
        ),
      );
      return Promise.resolve(false);
    }
    return saveWithToast(
      () => save.mutateAsync(payloadFromDraft(draft)),
      t("settings.providers.modelsSaved", "Models saved"),
    );
  };

  const addDisabled =
    !enabled ||
    save.isPending ||
    newId.trim() === "" ||
    (newKind === "embedding" && !(Number.parseInt(newDimension, 10) > 0));

  return (
    <div
      className="flex w-full flex-col gap-2"
      data-testid={`fleet-models-${slug}`}
    >
      <p className="text-sm font-medium">{t("settings.providers.modelsHeading", "Models")}</p>
      {!enabled && canTest ? (
        <p className="text-sm text-muted-foreground" data-testid={`test-first-${slug}`}>
          {t(
            "settings.providers.testBeforeModels",
            "Test the connection above. A passing test offers this provider to every organization, and then these models can be saved.",
          )}
        </p>
      ) : null}
      {!enabled && !canTest ? (
        <p className="text-xs text-muted-foreground" data-testid={`test-first-${slug}`}>
          {t(
            "settings.providers.fixConnection",
            "Models can be registered once the connection test passes.",
          )}{" "}
          <Link href={sectionHref("connections")} className="underline">
            {t("settings.providers.openConnections", "Open Connections")}
          </Link>
        </p>
      ) : null}
      {draft.length > 0 ? (
        <ul
          className="flex flex-col gap-1"
          aria-label={t("settings.providers.registered", "Registered models")}
        >
          {draft.map((entry) => (
            <li
              key={entry.id}
              className="flex flex-wrap items-center gap-2 rounded-md border bg-muted/30 px-2 py-1 text-sm"
              data-testid={`fleet-model-${slug}-${entry.id}`}
            >
              <span
                className="min-w-0 flex-1 truncate font-mono text-xs"
                title={entry.id}
              >
                {shortModelId(entry.id)}
              </span>
              <CapabilityChips entry={entry} />
              {entry.kind === "embedding" ? (
                <Input
                  type="number"
                  min={1}
                  inputMode="numeric"
                  className="h-7 w-20 text-xs"
                  aria-label={t(
                    "settings.providers.dimensionFor",
                    "Dimension for {{id}}",
                    { id: entry.id },
                  )}
                  aria-invalid={!(entry.dimension && entry.dimension > 0)}
                  value={entry.dimension ?? ""}
                  placeholder="dim"
                  disabled={!enabled || save.isPending}
                  onChange={(event) =>
                    setDimension(entry.id, event.target.value)
                  }
                />
              ) : null}
              {enabled ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  disabled={save.isPending}
                  aria-label={t(
                    "settings.providers.removeModel",
                    "Remove {{id}}",
                    { id: entry.id },
                  )}
                  onClick={() =>
                    setDraft((current) =>
                      current.filter((item) => item.id !== entry.id),
                    )
                  }
                >
                  <X className="h-3.5 w-3.5" aria-hidden />
                </Button>
              ) : null}
            </li>
          ))}
        </ul>
      ) : enabled ? (
        <p className="text-xs text-muted-foreground">
          {t(
            "settings.providers.noModels",
            "No model is registered yet. Refresh to discover some, or add an id.",
          )}
        </p>
      ) : null}
      {canRefresh ? (
        <Button
          type="button"
          size="sm"
          variant="outline"
          className="self-start"
          disabled={refreshing || save.isPending}
          data-testid={`fleet-refresh-${slug}`}
          onClick={() => void refresh()}
        >
          {refreshing
            ? t("settings.providers.refreshing", "Refreshing…")
            : t("settings.providers.refresh", "Refresh models")}
        </Button>
      ) : null}
      {refreshError ? (
        <p
          className="text-xs text-destructive"
          role="alert"
          data-testid={`fleet-refresh-error-${slug}`}
        >
          {refreshError}
        </p>
      ) : null}
      {discovered.length > 0 ? (
        <ul
          className="flex max-h-40 flex-col gap-1 overflow-auto"
          data-testid={`fleet-discovered-${slug}`}
        >
          {discovered.map((item) => (
            <li key={item.name} className="flex items-center gap-2 text-sm">
              <Checkbox
                id={`${slug}-${item.name}`}
                checked={draft.some((entry) => entry.id === item.name)}
                disabled={save.isPending}
                onCheckedChange={(checked) =>
                  toggleDiscovered(item, checked === true)
                }
                aria-label={item.display_name || item.name}
              />
              <label
                htmlFor={`${slug}-${item.name}`}
                className="min-w-0 truncate"
              >
                {item.display_name || item.name}
              </label>
            </li>
          ))}
        </ul>
      ) : null}
      {enabled ? (
        <div className="flex flex-wrap items-center gap-2">
          <Input
            id={fieldId}
            aria-label={t(
              "settings.providers.modelIdFor",
              "Model id for {{slug}}",
              { slug },
            )}
            className="min-w-0 flex-1 basis-40"
            value={newId}
            disabled={save.isPending}
            placeholder={t("settings.providers.modelId", "Model id")}
            onChange={(event) => setNewId(event.target.value)}
          />
          <select
              aria-label={t("settings.providers.kindFor", "Type for {{slug}}", {
                slug,
              })}
              className="h-9 rounded-md border bg-background px-2 text-sm"
              disabled={save.isPending}
              value={newKind}
              onChange={(event) =>
                setNewKind(
                  event.target.value === "embedding" ? "embedding" : "chat",
                )
              }
            >
              <option value="chat">
                {t("settings.providers.cap.chat", "chat")}
              </option>
              <option value="embedding">
                {t("settings.providers.cap.embedding", "embedding")}
              </option>
            </select>
            {newKind === "embedding" ? (
              <Input
                type="number"
                min={1}
                inputMode="numeric"
                className="w-24"
                aria-label={t("settings.providers.newDimension", "Dimension")}
                placeholder="dim"
                value={newDimension}
                onChange={(event) => setNewDimension(event.target.value)}
              />
            ) : null}
            <Button
              type="button"
              size="sm"
              variant="outline"
              disabled={addDisabled}
              data-testid={`add-model-${slug}`}
              onClick={addEntry}
            >
              {t("settings.providers.addModel", "Add model")}
            </Button>
        </div>
      ) : null}
      {missing.length > 0 ? (
        <p className="text-xs text-amber-700 dark:text-amber-400" role="status">
          {t(
            "settings.providers.dimensionRequired",
            "An embedding model needs its dimension.",
          )}
        </p>
      ) : null}
      {enabled ? (
        <ModelSaveBar
          dirty={dirty}
          saving={save.isPending}
          onSave={() => void saveModels()}
          onDiscard={reset}
        />
      ) : null}
    </div>
  );
}
