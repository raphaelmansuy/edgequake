"use client";

import {
  CHOICE_CHECK,
  CHOICE_HEADING,
  CHOICE_MUTED,
  CHOICE_POPOVER,
  CHOICE_ROW,
} from "@/components/models/choice-menu-surface";
import { ProviderIcon } from "@/components/providers/provider-icon";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Switch } from "@/components/ui/switch";
import { useLlmCapabilityModels } from "@/hooks/use-llm-capability-models";
import { useQueryChoice } from "@/hooks/use-query-choice";
import type { EligibleModel } from "@/lib/api/edgequake/model-catalog";
import type { ModelResponse } from "@/lib/api/models";
import { modelSupportsThinking } from "@/lib/settings/reasoning-effort-supported";
import {
  formatFullId,
  type ModelMenuItem,
} from "@/lib/query/model-menu";
import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/use-settings-store";
import { useTenantStore } from "@/stores/use-tenant-store";
import { Check, ChevronDown, Eye, Loader2, Lock, Radio, Sparkles } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useShallow } from "zustand/react/shallow";

interface ModelChipProps {
  className?: string;
}

/**
 * Composer model switcher: one click → searchable list grouped by provider,
 * with a streaming toggle that reflects what the selected model supports.
 */
export function ModelChip({ className }: ModelChipProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const { eligible, queryUsage, models: visible, locked, empty, inheritOk } = useQueryChoice();
  const { isLoading, isError } = eligible;
  const { provider, model, stream } = useSettingsStore(
    useShallow((s) => {
      const slot = workspaceId ? s.queryModelsByWorkspace[workspaceId] : undefined;
      return {
        provider: workspaceId ? slot?.provider : s.querySettings.provider,
        model: workspaceId ? slot?.model : s.querySettings.model,
        stream: s.querySettings.stream,
      };
    }),
  );
  const setQuerySettings = useSettingsStore((s) => s.setQuerySettings);
  const setWorkspaceQueryModel = useSettingsStore((s) => s.setWorkspaceQueryModel);

  const currentFullId = provider && model ? formatFullId(provider, model) : "";
  const inherited = inheritOk
    ? visible.find(
        (entry) => entry.provider === queryUsage?.provider && entry.model === queryUsage?.model,
      )
    : undefined;
  const inheritLabel = inherited?.display_name
    || (inheritOk ? queryUsage?.model : undefined)
    || t("query.model.inherit", "Inherit");
  const pickLabel = t("query.model.pick", "Pick a model");
  const effectiveProvider = provider
    ?? (inheritOk ? queryUsage?.provider : undefined);

  const capabilityModels = useLlmCapabilityModels();
  const groups = useMemo(
    () => groupEligible(visible, search, capabilityModels),
    [visible, search, capabilityModels],
  );

  const selected = visible.find((entry) => entry.provider === provider && entry.model === model);
  const modelStreams = provider && model
    ? selected?.supports_streaming !== false
    : inheritOk
      ? inherited?.supports_streaming !== false
      : true;

  const triggerLabel = provider && model
    ? (selected?.display_name || model)
    : inheritOk
      ? inheritLabel
      : pickLabel;

  const select = (item: { provider: string; name: string } | null) => {
    const pair = item
      ? { provider: item.provider, model: item.name }
      : { provider: undefined, model: undefined };
    if (workspaceId) setWorkspaceQueryModel(workspaceId, pair);
    else setQuerySettings(pair);
    setOpen(false);
    setSearch("");
  };

  if (locked) {
    const lockedLabel = queryUsage?.model || triggerLabel;
    return (
      <Button
        type="button"
        variant="ghost"
        size="sm"
        disabled
        className={cn(
          "h-8 w-44 max-w-full min-w-0 shrink justify-start gap-1.5 px-2 text-[13px] font-medium",
          "text-[#57544e] disabled:opacity-100 dark:text-[#b5b2ab]",
          className,
        )}
        data-testid="query-model-chip"
        data-locked="true"
        aria-label={t("query.model.locked", "Locked")}
        title={t("query.model.lockedHint", "An operator lock chooses this model")}
      >
        <Lock className="h-3.5 w-3.5 shrink-0 text-[#8f5e0e] dark:text-[#d8a443]" aria-hidden />
        <span className="min-w-0 flex-1 truncate text-left">{lockedLabel}</span>
        <span className="shrink-0 text-[11px] font-semibold uppercase tracking-[0.06em] text-[#8f5e0e] dark:text-[#d8a443]">
          {t("query.model.locked", "Locked")}
        </span>
      </Button>
    );
  }

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) setSearch("");
      }}
    >
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className={cn(
            "h-8 w-44 max-w-full min-w-0 shrink justify-start overflow-hidden gap-1.5 px-2 text-[13px] font-medium",
            "text-[#57544e] hover:bg-[#f6f5f2] hover:text-[#1d1c1a] dark:text-[#b5b2ab] dark:hover:bg-[#252524] dark:hover:text-[#eceae5]",
            open && "bg-[#f6f5f2] text-[#1d1c1a] dark:bg-[#252524] dark:text-[#eceae5]",
            className,
          )}
          data-testid="query-model-chip"
          aria-label={t("query.model.change", "Change model")}
          aria-haspopup="listbox"
          aria-expanded={open}
          title={provider && model ? `${provider}/${model}` : triggerLabel}
        >
          {effectiveProvider ? (
            <ProviderIcon providerId={effectiveProvider} className="h-3.5 w-3.5 shrink-0" />
          ) : (
            <Sparkles className="h-3.5 w-3.5 shrink-0 opacity-70" aria-hidden />
          )}
          <span className="min-w-0 flex-1 truncate text-left">{triggerLabel}</span>
          {eligible.data && !modelStreams ? (
            <span
              className="h-1.5 w-1.5 shrink-0 rounded-full bg-amber-500"
              aria-hidden
              title={t("query.model.noStreaming", "No live streaming")}
            />
          ) : null}
          <ChevronDown
            className={cn(
              "h-3 w-3 shrink-0 opacity-60 transition-transform duration-200",
              open && "rotate-180",
            )}
            aria-hidden
          />
        </Button>
      </PopoverTrigger>

      <PopoverContent
        align="start"
        side="top"
        sideOffset={10}
        collisionPadding={12}
        className={cn(CHOICE_POPOVER, "w-[320px] p-1")}
        data-testid="query-model-menu"
      >
        <Command shouldFilter={false} className="rounded-none bg-transparent">
          <CommandInput
            value={search}
            onValueChange={setSearch}
            placeholder={t("query.model.search", "Search models…")}
            data-testid="query-model-search"
            aria-label={t("query.model.search", "Search models…")}
          />
          <CommandList className={cn("max-h-[300px]", CHOICE_HEADING)}>
            {isLoading ? (
              <div className={cn("flex items-center gap-2 px-2 py-3 text-[12.5px]", CHOICE_MUTED)}>
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                {t("common.loading", "Loading…")}
              </div>
            ) : null}
            {isError ? (
              <div className={cn("px-2 py-3 text-[12.5px]", CHOICE_MUTED)}>
                {t("query.model.loadError", "Could not load models.")}
              </div>
            ) : null}

            {!isLoading && !isError && !search && inheritOk ? (
              <CommandGroup>
                <CommandItem
                  value="__server_default__"
                  onSelect={() => select(null)}
                  data-testid="query-model-option-default"
                  className={CHOICE_ROW}
                >
                  <Sparkles className="h-4 w-4" aria-hidden />
                  <div className="min-w-0 flex-1">
                    <div className="text-[13.5px] font-medium">
                      {t("query.model.inherit", "Inherit")}
                    </div>
                    {inherited || queryUsage?.model ? (
                      <div className={cn("truncate text-[12px]", CHOICE_MUTED)}>
                        {inheritLabel}
                      </div>
                    ) : null}
                  </div>
                  {!currentFullId ? <Check className={CHOICE_CHECK} /> : null}
                </CommandItem>
              </CommandGroup>
            ) : null}

            {!isLoading && !isError && !search && !inheritOk && !empty ? (
              <p
                className={cn("px-2 py-2 text-[12.5px]", CHOICE_MUTED)}
                data-testid="query-choice-pick-hint"
              >
                {t(
                  "query.model.pickHint",
                  "The workspace default isn’t available here. Choose a model below.",
                )}
              </p>
            ) : null}

            {!isLoading && !isError && empty && !search ? (
              <p className={cn("px-2 py-2 text-[12.5px]", CHOICE_MUTED)} data-testid="query-choice-empty-menu">
                {t(
                  "query.model.emptyEligible",
                  "An admin needs to allow a model before you can ask.",
                )}
              </p>
            ) : null}

            {!isLoading && !isError && groups.length === 0 && !empty ? (
              <CommandEmpty>
                {t("query.model.noResults", "No models match “{{query}}”", {
                  query: search,
                })}
              </CommandEmpty>
            ) : null}

            {groups.map((group) => (
              <CommandGroup key={group.provider} heading={group.displayName}>
                {group.items.map((item) => (
                  <ModelRow
                    key={item.fullId}
                    item={item}
                    selected={item.fullId === currentFullId}
                    onSelect={() => select(item)}
                  />
                ))}
              </CommandGroup>
            ))}
          </CommandList>
        </Command>

        <p className={cn("px-2 pb-1 text-[11px]", CHOICE_MUTED)} data-testid="query-model-saved-locally">
          {t("query.model.savedLocally", "Saved in this browser")}
        </p>
        <div className="mt-1 flex items-center justify-between gap-3 border-t border-[#f0eeea] px-2 py-2 dark:border-[#252523]">
          <label
            htmlFor="query-model-stream"
            className="flex min-w-0 items-center gap-2 text-[12.5px]"
          >
            <Radio className="h-3.5 w-3.5 shrink-0 text-muted-foreground" aria-hidden />
            <span className="min-w-0">
              <span className="block font-medium">
                {t("query.model.stream", "Stream answer")}
              </span>
              <span className={cn("block truncate", CHOICE_MUTED)}>
                {modelStreams
                  ? t("query.model.streamHint", "Tokens appear as they are written")
                  : t(
                      "query.model.noStreamHint",
                      "This model can’t stream — answers arrive at once",
                    )}
              </span>
            </span>
          </label>
          <Switch
            id="query-model-stream"
            checked={stream && modelStreams}
            disabled={!modelStreams}
            onCheckedChange={(checked) => setQuerySettings({ stream: checked })}
            data-testid="query-model-stream-toggle"
          />
        </div>
      </PopoverContent>
    </Popover>
  );
}

/**
 * Group the eligible models by provider. Capabilities come from the catalog, because the
 * eligible list carries only what the picker needs to choose a model.
 */
function groupEligible(
  models: EligibleModel[],
  search: string,
  capabilityModels: ModelResponse[] | undefined,
) {
  const query = search.trim().toLowerCase();
  const groups = new Map<string, { provider: string; displayName: string; items: ModelMenuItem[] }>();
  for (const model of models) {
    const hay = `${model.display_name} ${model.model} ${model.provider_display_name}`.toLowerCase();
    if (query && !query.split(/\s+/).every((token) => hay.includes(token))) continue;
    let group = groups.get(model.provider);
    if (!group) {
      group = {
        provider: model.provider,
        displayName: model.provider_display_name || model.provider,
        items: [],
      };
      groups.set(model.provider, group);
    }
    group.items.push({
      fullId: formatFullId(model.provider, model.model),
      provider: model.provider,
      name: model.model,
      displayName: model.display_name || model.model,
      supportsStreaming: model.supports_streaming !== false,
      supportsVision: model.supports_vision,
      supportsThinking:
        modelSupportsThinking(capabilityModels, model.provider, model.model) === true,
      deprecated: false,
      available: true,
    });
  }
  return [...groups.values()];
}

function ModelRow({
  item,
  selected,
  onSelect,
}: {
  item: ModelMenuItem;
  selected: boolean;
  onSelect: () => void;
}) {
  const { t } = useTranslation();
  return (
    <CommandItem
      value={item.fullId}
      onSelect={onSelect}
      data-testid="query-model-option"
      data-model={item.fullId}
      className={CHOICE_ROW}
    >
      <div className="min-w-0 flex-1">
        <div className="truncate text-[13.5px] font-medium">{item.displayName}</div>
        {item.displayName !== item.name ? (
          <div className={cn("truncate text-[12px]", CHOICE_MUTED)}>{item.name}</div>
        ) : null}
      </div>
      <div className="flex shrink-0 items-center gap-1.5 text-muted-foreground">
        {item.supportsVision ? (
          <Eye
            className="h-3.5 w-3.5"
            aria-label={t("query.model.vision", "Vision")}
          />
        ) : null}
        {item.supportsThinking ? (
          <Sparkles
            className="h-3.5 w-3.5"
            aria-label={t("query.model.thinking", "Reasoning")}
          />
        ) : null}
        {!item.supportsStreaming ? (
          <span
            className="rounded bg-amber-500/15 px-1 text-[10px] font-medium text-amber-700 dark:text-amber-400"
            title={t("query.model.noStreaming", "No live streaming")}
          >
            {t("query.model.noStreamBadge", "no stream")}
          </span>
        ) : null}
        {selected ? <Check className={CHOICE_CHECK} aria-hidden /> : null}
      </div>
    </CommandItem>
  );
}
