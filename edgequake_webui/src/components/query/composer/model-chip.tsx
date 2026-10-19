"use client";

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
import { useLlmModels } from "@/hooks/use-providers";
import {
  buildModelMenu,
  formatFullId,
  modelSupportsStreaming,
  modelTriggerLabel,
  type ModelMenuItem,
} from "@/lib/query/model-menu";
import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/use-settings-store";
import { Check, ChevronDown, Eye, Loader2, Radio, Sparkles } from "lucide-react";
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
  const { data, isLoading, isError } = useLlmModels();
  const { provider, model, stream } = useSettingsStore(
    useShallow((s) => ({
      provider: s.querySettings.provider,
      model: s.querySettings.model,
      stream: s.querySettings.stream,
    })),
  );
  const setQuerySettings = useSettingsStore((s) => s.setQuerySettings);

  const currentFullId = provider && model ? formatFullId(provider, model) : "";
  const defaultFullId =
    data?.default_provider && data?.default_model
      ? formatFullId(data.default_provider, data.default_model)
      : "";
  const effectiveProvider = provider ?? data?.default_provider;

  const groups = useMemo(
    () =>
      buildModelMenu(data?.models ?? [], {
        query: search,
        currentFullId: currentFullId || defaultFullId,
      }),
    [data?.models, search, currentFullId, defaultFullId],
  );

  const modelStreams = modelSupportsStreaming(data?.models, {
    provider,
    model,
    defaultProvider: data?.default_provider,
    defaultModel: data?.default_model,
  });

  const triggerLabel = modelTriggerLabel(
    { provider, model },
    data?.default_model ?? t("query.model.default", "Default model"),
  );

  const select = (item: ModelMenuItem | null) => {
    setQuerySettings(
      item
        ? { provider: item.provider, model: item.name }
        : { provider: undefined, model: undefined },
    );
    setOpen(false);
    setSearch("");
  };

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
            "h-8 w-44 max-w-full min-w-0 shrink-0 justify-start overflow-hidden gap-1.5 px-2 text-xs font-medium",
            "text-muted-foreground hover:text-foreground",
            open && "bg-muted/70 text-foreground",
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
          {data && !modelStreams ? (
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
        className="w-[340px] overflow-hidden rounded-xl border-border/60 p-0 shadow-xl"
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
          <CommandList className="max-h-[300px] p-1">
            {isLoading ? (
              <div className="flex items-center gap-2 px-3 py-4 text-xs text-muted-foreground">
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                {t("common.loading", "Loading…")}
              </div>
            ) : null}
            {isError ? (
              <div className="px-3 py-4 text-xs text-muted-foreground">
                {t(
                  "query.model.loadError",
                  "Could not load models. The server default will be used.",
                )}
              </div>
            ) : null}

            {!isLoading && !isError && !search ? (
              <CommandGroup>
                <CommandItem
                  value="__server_default__"
                  onSelect={() => select(null)}
                  data-testid="query-model-option-default"
                  className="rounded-lg py-2"
                >
                  <Sparkles className="h-4 w-4" aria-hidden />
                  <div className="min-w-0 flex-1">
                    <div className="text-sm font-medium">
                      {t("query.model.serverDefault", "Server default")}
                    </div>
                    {defaultFullId ? (
                      <div className="truncate text-xs text-muted-foreground">
                        {defaultFullId}
                      </div>
                    ) : null}
                  </div>
                  {!currentFullId ? <Check className="h-4 w-4 text-primary" /> : null}
                </CommandItem>
              </CommandGroup>
            ) : null}

            {!isLoading && !isError && groups.length === 0 ? (
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

        <div className="flex items-center justify-between gap-3 border-t bg-muted/30 px-3 py-2.5">
          <label
            htmlFor="query-model-stream"
            className="flex min-w-0 items-center gap-2 text-xs"
          >
            <Radio className="h-3.5 w-3.5 shrink-0 text-muted-foreground" aria-hidden />
            <span className="min-w-0">
              <span className="block font-medium">
                {t("query.model.stream", "Stream answer")}
              </span>
              <span className="block truncate text-muted-foreground">
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
      className="rounded-lg py-2"
    >
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium">{item.displayName}</div>
        {item.displayName !== item.name ? (
          <div className="truncate text-xs text-muted-foreground">{item.name}</div>
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
        {selected ? <Check className="h-4 w-4 text-primary" aria-hidden /> : null}
      </div>
    </CommandItem>
  );
}
