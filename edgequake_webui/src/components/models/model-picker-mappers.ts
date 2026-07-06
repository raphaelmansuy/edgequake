/**
 * Shared mappers for ModelPickerPanel (SPEC-043 DRY).
 */
import type { EmbeddingModelItem, LlmModelItem } from "@/lib/api/models";
import { getProviderDisplayName } from "@/lib/provider-display";
import type { ModelPickerOption } from "./model-picker-panel";
import { formatModelFullId, parseModelFullId } from "./model-picker-panel";
import { mergePickerOptions } from "./model-picker-options";

/** API shape from GET /models/search hits. */
export interface ModelSearchHit {
  provider: string;
  id: string;
  name: string;
  context_length: number;
  supports_vision: boolean;
  supports_tools: boolean;
  discovery_source?: string;
  available?: boolean;
}

export function searchHitToPickerOption(hit: ModelSearchHit): ModelPickerOption {
  return {
    provider: hit.provider,
    providerDisplayName: getProviderDisplayName(hit.provider),
    name: hit.id,
    displayName: hit.name,
    fullId: formatModelFullId(hit.provider, hit.id),
    contextLength: hit.context_length,
    supportsVision: hit.supports_vision,
    supportsTools: hit.supports_tools,
    supportsStreaming: true,
    isLive: isLiveDiscoveredSource(hit.discovery_source),
    available: hit.available,
  };
}

/** Map search hits to deduped picker options. */
export function searchHitsToPickerOptions(hits: ReadonlyArray<ModelSearchHit>): ModelPickerOption[] {
  return mergePickerOptions(hits.map(searchHitToPickerOption));
}

/** Tenant/API legacy format: `provider:model` (colon). Picker uses `provider/model`. */
export function colonValueToFullId(value: string): string {
  const colonIndex = value.indexOf(":");
  if (colonIndex === -1) return value;
  return formatModelFullId(
    value.substring(0, colonIndex),
    value.substring(colonIndex + 1),
  );
}

export function fullIdToColonValue(fullId: string): string {
  const { provider, model } = parseModelFullId(fullId);
  return `${provider}:${model}`;
}

export function isLiveDiscoveredSource(source?: string): boolean {
  return source === "dynamic_api" || source === "hybrid";
}

export function llmModelToPickerOption(model: LlmModelItem): ModelPickerOption {
  return {
    provider: model.provider,
    providerDisplayName: model.provider_display_name,
    name: model.name,
    displayName: model.display_name,
    fullId: formatModelFullId(model.provider, model.name),
    contextLength: model.capabilities.context_length,
    supportsVision: model.capabilities.supports_vision,
    supportsTools: model.capabilities.supports_function_calling,
    supportsStreaming: model.capabilities.supports_streaming,
    deprecated: model.deprecated,
    isLive: isLiveDiscoveredSource(model.discovery_source),
    available: model.available,
  };
}

export function embeddingModelToPickerOption(model: EmbeddingModelItem): ModelPickerOption {
  return {
    provider: model.provider,
    providerDisplayName: model.provider_display_name,
    name: model.name,
    displayName: model.display_name,
    fullId: formatModelFullId(model.provider, model.name),
    contextLength: 0,
    supportsVision: false,
    supportsTools: false,
    supportsStreaming: false,
    dimension: model.dimension,
    deprecated: model.deprecated,
    isLive: isLiveDiscoveredSource(model.discovery_source),
    available: model.available,
  };
}
