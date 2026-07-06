/**
 * Unified model picker with provider filter, search, and capability chips (SPEC-043).
 */
"use client";

import { ProviderIcon } from "@/components/providers/provider-icon";
import { Badge } from "@/components/ui/badge";
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
import { cn } from "@/lib/utils";
import { getProviderDisplayName } from "@/lib/provider-display";
import { apiClient } from "@/lib/api/client";
import {
  searchHitsToPickerOptions,
  type ModelSearchHit,
} from "@/components/models/model-picker-mappers";
import {
  ensureSelectedInPickerOptions,
  groupPickerOptionsByProvider,
  mergePickerOptions,
} from "@/components/models/model-picker-options";
import { Check, ChevronDown, Eye, Loader2, Wrench, Zap } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  useScrollContainedWheel,
  useScrollSelectedIntoView,
} from "@/hooks/use-scroll-contained-wheel";

export interface ModelPickerValue {
  provider: string;
  model: string;
  fullId: string;
}

export interface ModelPickerOption {
  provider: string;
  providerDisplayName: string;
  name: string;
  displayName: string;
  fullId: string;
  contextLength: number;
  supportsVision: boolean;
  supportsTools: boolean;
  supportsStreaming: boolean;
  /** Embedding models: vector dimension (shown instead of context length). */
  dimension?: number;
  deprecated?: boolean;
  /** Model was discovered via live provider API (Ollama, LM Studio, OpenRouter, …). */
  isLive?: boolean;
  /** Runtime availability from provider discovery. */
  available?: boolean;
}

interface ModelSearchResponse {
  hits: ModelSearchHit[];
  total: number;
}

export interface ModelPickerPanelProps {
  options: ModelPickerOption[];
  value?: ModelPickerValue;
  onChange?: (value: ModelPickerValue | undefined) => void;
  disabled?: boolean;
  className?: string;
  placeholder?: string;
  allowServerDefault?: boolean;
  serverDefaultLabel?: string;
  filterVision?: boolean;
  isLoading?: boolean;
  /** LLM shows capability chips + remote search; embedding shows dimensions only. */
  variant?: "llm" | "embedding";
  showProviderFilters?: boolean;
  showCapabilityFilters?: boolean;
  enableRemoteSearch?: boolean;
  testId?: string;
}

type CapabilityFilter = "vision" | "tools" | "streaming";

export function formatModelFullId(provider: string, model: string): string {
  return `${provider}/${model}`;
}

export function parseModelFullId(fullId: string): { provider: string; model: string } {
  const slashIndex = fullId.indexOf("/");
  if (slashIndex === -1) {
    return { provider: "unknown", model: fullId };
  }
  return {
    provider: fullId.substring(0, slashIndex),
    model: fullId.substring(slashIndex + 1),
  };
}

export function ModelPickerPanel({
  options,
  value,
  onChange,
  disabled,
  className,
  placeholder = "Select model…",
  allowServerDefault = true,
  serverDefaultLabel = "Server default",
  filterVision = false,
  isLoading = false,
  variant = "llm",
  showProviderFilters = true,
  showCapabilityFilters,
  enableRemoteSearch,
  testId = "model-picker-panel",
}: ModelPickerPanelProps) {
  const isEmbedding = variant === "embedding";
  const showCaps = showCapabilityFilters ?? (!isEmbedding && !filterVision);
  const remoteSearch = enableRemoteSearch ?? !isEmbedding;
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [providerFilter, setProviderFilter] = useState<string | null>(null);
  const [capabilityFilters, setCapabilityFilters] = useState<Set<CapabilityFilter>>(new Set());
  const [remoteHits, setRemoteHits] = useState<ModelSearchHit[] | null>(null);
  const [providerCatalogHits, setProviderCatalogHits] = useState<ModelSearchHit[] | null>(null);
  const [searchLoading, setSearchLoading] = useState(false);
  const [providerCatalogLoading, setProviderCatalogLoading] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const { onWheel, className: scrollContainedClass } = useScrollContainedWheel();
  useScrollSelectedIntoView(listRef, open);

  const buildDisplayOptions = useCallback(
    (
      primary: ModelPickerOption[],
      secondary: ModelPickerOption[] = [],
    ): ModelPickerOption[] =>
      ensureSelectedInPickerOptions(
        mergePickerOptions(primary, secondary),
        value,
        options,
      ),
    [options, value],
  );

  // Keep provider chip in sync with the current selection.
  useEffect(() => {
    if (value?.provider) {
      setProviderFilter(value.provider);
    }
  }, [value?.provider]);

  // Dynamic provider-scoped catalog (OpenRouter, Ollama live tags, …).
  useEffect(() => {
    if (!open || !remoteSearch || !providerFilter || search.trim().length >= 2) {
      setProviderCatalogHits(null);
      return;
    }

    let cancelled = false;
    const timer = setTimeout(async () => {
      setProviderCatalogLoading(true);
      try {
        const params = new URLSearchParams({
          provider: providerFilter,
          limit: "50",
        });
        if (filterVision || capabilityFilters.has("vision")) {
          params.set("requires_vision", "true");
        }
        if (capabilityFilters.has("tools")) {
          params.set("requires_tools", "true");
        }
        const data = await apiClient<ModelSearchResponse>(
          `/models/search?${params.toString()}`,
        );
        if (!cancelled) setProviderCatalogHits(data.hits);
      } catch {
        if (!cancelled) setProviderCatalogHits(null);
      } finally {
        if (!cancelled) setProviderCatalogLoading(false);
      }
    }, 150);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [open, providerFilter, capabilityFilters, filterVision, remoteSearch, search]);

  const providers = useMemo(() => {
    const map = new Map<string, number>();
    for (const opt of options) {
      map.set(opt.provider, (map.get(opt.provider) ?? 0) + 1);
    }
    return Array.from(map.entries())
      .map(([id, count]) => ({
        id,
        displayName: getProviderDisplayName(id),
        count,
      }))
      .sort((a, b) => a.displayName.localeCompare(b.displayName));
  }, [options]);

  useEffect(() => {
    if (!open || !remoteSearch) return;
    const q = search.trim();
    if (q.length < 2) {
      setRemoteHits(null);
      return;
    }

    const timer = setTimeout(async () => {
      setSearchLoading(true);
      try {
        const params = new URLSearchParams({ q, fuzzy: "true", limit: "40" });
        if (providerFilter) params.set("provider", providerFilter);
        if (filterVision || capabilityFilters.has("vision")) {
          params.set("requires_vision", "true");
        }
        if (capabilityFilters.has("tools")) {
          params.set("requires_tools", "true");
        }
        const data = await apiClient<ModelSearchResponse>(
          `/models/search?${params.toString()}`,
        );
        setRemoteHits(data.hits);
      } catch {
        setRemoteHits(null);
      } finally {
        setSearchLoading(false);
      }
    }, 200);

    return () => clearTimeout(timer);
  }, [search, providerFilter, capabilityFilters, filterVision, open, remoteSearch]);

  const filteredOptions = useMemo(() => {
    let list = options;
    if (filterVision) list = list.filter((o) => o.supportsVision);
    if (providerFilter) list = list.filter((o) => o.provider === providerFilter);
    if (capabilityFilters.has("vision")) list = list.filter((o) => o.supportsVision);
    if (capabilityFilters.has("tools")) list = list.filter((o) => o.supportsTools);
    if (capabilityFilters.has("streaming")) list = list.filter((o) => o.supportsStreaming);
    const q = search.trim().toLowerCase();
    if (q && !remoteHits) {
      list = list.filter(
        (o) =>
          o.name.toLowerCase().includes(q) ||
          o.displayName.toLowerCase().includes(q) ||
          o.provider.toLowerCase().includes(q),
      );
    }
    return mergePickerOptions(list);
  }, [options, providerFilter, capabilityFilters, filterVision, search, remoteHits]);

  const displayOptions = useMemo(() => {
    if (remoteHits && search.trim().length >= 2) {
      return buildDisplayOptions(searchHitsToPickerOptions(remoteHits));
    }
    if (providerCatalogHits && providerFilter) {
      // Live catalog + static slice for same provider (deduped).
      return buildDisplayOptions(
        searchHitsToPickerOptions(providerCatalogHits),
        filteredOptions,
      );
    }
    return buildDisplayOptions(filteredOptions);
  }, [
    remoteHits,
    search,
    providerCatalogHits,
    providerFilter,
    filteredOptions,
    buildDisplayOptions,
  ]);

  const grouped = useMemo(
    () => groupPickerOptionsByProvider(displayOptions, value?.provider ?? providerFilter),
    [displayOptions, value?.provider, providerFilter],
  );

  const toggleCapability = useCallback((cap: CapabilityFilter) => {
    setCapabilityFilters((prev) => {
      const next = new Set(prev);
      if (next.has(cap)) next.delete(cap);
      else next.add(cap);
      return next;
    });
  }, []);

  const handleSelect = (fullId: string) => {
    if (fullId === "default") {
      onChange?.(undefined);
    } else {
      const { provider, model } = parseModelFullId(fullId);
      onChange?.({ provider, model, fullId });
    }
    setOpen(false);
  };

  const capabilityChoices: CapabilityFilter[] = filterVision
    ? ["vision"]
    : ["vision", "tools", "streaming"];

  const selectedDimension = value
    ? options.find((o) => o.fullId === value.fullId)?.dimension
    : undefined;

  const formatOptionSubline = (opt: ModelPickerOption) => {
    if (isEmbedding && opt.dimension) {
      return `${opt.name} · ${opt.dimension}d`;
    }
    if (opt.contextLength > 0) {
      return `${opt.name} · ${(opt.contextLength / 1000).toFixed(0)}K ctx`;
    }
    return opt.name;
  };

  return (
    <div className={cn("space-y-2", className)} data-testid={testId}>
      {showProviderFilters && providers.length > 0 && (
      <div className="flex flex-wrap gap-1.5" data-testid="model-picker-provider-bar">
        <Button
          type="button"
          variant={providerFilter === null ? "secondary" : "outline"}
          size="sm"
          className="h-7 text-xs"
          data-testid="model-picker-provider-all"
          onClick={() => setProviderFilter(null)}
        >
          All providers
        </Button>
        {providers.map((p) => (
          <Button
            key={p.id}
            type="button"
            variant={providerFilter === p.id ? "secondary" : "outline"}
            size="sm"
            className="h-7 text-xs gap-1"
            data-testid={`model-picker-provider-${p.id}`}
            onClick={() => setProviderFilter(providerFilter === p.id ? null : p.id)}
          >
            <ProviderIcon providerId={p.id} className="h-3 w-3" />
            {p.displayName}
            <span className="opacity-60">({p.count})</span>
          </Button>
        ))}
      </div>
      )}

      {showCaps && (
      <div className="flex flex-wrap gap-1.5" data-testid="model-picker-capability-bar">
        {capabilityChoices.map((cap) => (
          <Badge
            key={cap}
            variant={capabilityFilters.has(cap) ? "default" : "outline"}
            className="cursor-pointer capitalize"
            data-testid={`model-picker-capability-${cap}`}
            onClick={() => toggleCapability(cap)}
          >
            {cap}
          </Badge>
        ))}
      </div>
      )}

      <Popover
        open={open}
        onOpenChange={(next) => {
          setOpen(next);
          if (!next) {
            setSearch("");
            setRemoteHits(null);
            setProviderCatalogHits(null);
          }
        }}
      >
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            role="combobox"
            aria-expanded={open}
            disabled={disabled || isLoading}
            className="w-full justify-between font-normal"
            data-testid={`${testId}-trigger`}
          >
            {isLoading ? (
              <span className="flex items-center gap-2 text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                Loading models…
              </span>
            ) : value ? (
              <span className="flex items-center gap-2 truncate">
                <ProviderIcon providerId={value.provider} />
                <span className="truncate">{value.model}</span>
                {isEmbedding && selectedDimension ? (
                  <span className="text-xs text-muted-foreground shrink-0">
                    ({selectedDimension}d)
                  </span>
                ) : null}
              </span>
            ) : (
              <span className="text-muted-foreground">{serverDefaultLabel}</span>
            )}
            <ChevronDown className="h-4 w-4 shrink-0 opacity-50" />
          </Button>
        </PopoverTrigger>
        <PopoverContent
          className="w-[var(--radix-popover-trigger-width)] p-0"
          align="start"
          onOpenAutoFocus={(event) => {
            event.preventDefault();
            searchInputRef.current?.focus();
          }}
        >
          <Command shouldFilter={false} loop>
            <CommandInput
              ref={searchInputRef}
              placeholder={placeholder}
              value={search}
              onValueChange={setSearch}
              data-testid={`${testId}-search`}
            />
            <CommandList
              ref={listRef}
              className={cn("max-h-[320px] relative", scrollContainedClass)}
              onWheel={onWheel}
              data-testid={`${testId}-list`}
              aria-label="Model search results"
            >
              {(searchLoading || providerCatalogLoading) && (
                <div
                  className="sticky top-0 z-10 flex items-center justify-center border-b bg-popover/95 py-2 text-sm text-muted-foreground backdrop-blur-sm"
                  data-testid={`${testId}-list-loading`}
                >
                  <Loader2 className="h-4 w-4 animate-spin mr-2" />
                  {providerCatalogLoading && providerFilter
                    ? `Loading ${getProviderDisplayName(providerFilter)} models…`
                    : "Searching…"}
                </div>
              )}
              <CommandEmpty>
                {providerFilter
                  ? `No models for ${getProviderDisplayName(providerFilter)}. Type 2+ characters to search the full catalog.`
                  : "No models found. Select a provider or type 2+ characters to search."}
              </CommandEmpty>
              {allowServerDefault && (
                <CommandGroup>
                  <CommandItem value="default" onSelect={() => handleSelect("default")}>
                    <Check className={cn("mr-2 h-4 w-4", !value ? "opacity-100" : "opacity-0")} />
                    {serverDefaultLabel}
                  </CommandItem>
                </CommandGroup>
              )}
              {Array.from(grouped.entries()).map(([providerId, models]) => (
                <CommandGroup
                  key={providerId}
                  heading={
                    <span className="flex items-center gap-1">
                      <ProviderIcon providerId={providerId} className="h-3 w-3" />
                      {getProviderDisplayName(providerId)}
                    </span>
                  }
                >
                  {models.map((opt) => (
                    <CommandItem
                      key={opt.fullId}
                      value={opt.fullId}
                      disabled={opt.deprecated}
                      data-testid={`model-picker-option-${opt.fullId.replace(/\//g, "-")}`}
                      onSelect={() => handleSelect(opt.fullId)}
                    >
                      <Check
                        className={cn(
                          "mr-2 h-4 w-4",
                          value?.fullId === opt.fullId ? "opacity-100" : "opacity-0",
                        )}
                      />
                      <div className="flex flex-col min-w-0 flex-1">
                        <div className="flex items-center gap-1">
                          <span className="font-medium truncate">{opt.displayName}</span>
                          {opt.isLive && (
                            <Badge
                              variant="outline"
                              className="h-4 px-1 text-[10px] shrink-0"
                              data-testid="model-picker-live-badge"
                            >
                              Live
                            </Badge>
                          )}
                          {!isEmbedding && opt.supportsVision && (
                            <Eye className="h-3 w-3 text-blue-500" />
                          )}
                          {!isEmbedding && opt.supportsTools && (
                            <Wrench className="h-3 w-3 text-amber-500" />
                          )}
                          {!isEmbedding && opt.supportsStreaming && (
                            <Zap className="h-3 w-3 text-yellow-500" />
                          )}
                        </div>
                        <span className="text-xs text-muted-foreground truncate">
                          {formatOptionSubline(opt)}
                        </span>
                      </div>
                    </CommandItem>
                  ))}
                </CommandGroup>
              ))}
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
    </div>
  );
}
