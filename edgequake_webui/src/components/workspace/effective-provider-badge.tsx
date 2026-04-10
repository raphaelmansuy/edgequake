/**
 * @fileoverview EffectiveProviderBadge — shows the *actual* provider + model that will be used.
 *
 * ## First Principle
 *
 * Users must never have to guess which model is running. When a workspace has no override,
 * the server default is shown explicitly with a "server default" label. When the workspace
 * has a partial override (provider set but model unset, or vice-versa), each part is labelled
 * independently so the user can see exactly which layer each value comes from.
 *
 * ## Source-of-truth priority (highest → lowest)
 *
 * 1. Workspace explicit pair (workspace_provider + workspace_model — both set)
 * 2. Server default from /health (providers.{llm,embedding,vision})
 *
 * A partial workspace override (orphaned model without matching provider) is intentionally
 * NOT surfaced here — the backend fix (helpers.rs) already ignores it. We surface only the
 * resolved effective value.
 *
 * @implements SPEC-040: Explicit vision configuration
 * @enforces First Principle: no hidden defaults, no silent fallbacks visible to users
 */
'use client';

import { Badge } from '@/components/ui/badge';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { Server, Sparkles } from 'lucide-react';

export type ProviderSource = 'workspace' | 'server-default';

export interface EffectiveProviderConfig {
  /** Resolved provider name (e.g., "ollama", "openai"). */
  provider: string;
  /** Resolved model name. */
  model: string;
  /** Where this value comes from. */
  source: ProviderSource;
  /** Optional: embedding dimension (only for embedding configs). */
  dimension?: number;
}

interface EffectiveProviderBadgeProps {
  /** Title label shown above the badge row (e.g., "Extraction LLM", "Vision LLM"). */
  label: string;
  /** Resolved effective config to display. */
  config: EffectiveProviderConfig;
  /** Optional extra className for the wrapper div. */
  className?: string;
}

/**
 * Show provider + model in a compact badge row.
 * Source is indicated via icon and tooltip so the user always knows origin.
 */
export function EffectiveProviderBadge({
  label,
  config,
  className,
}: EffectiveProviderBadgeProps) {
  const isServerDefault = config.source === 'server-default';

  const tooltipText = isServerDefault
    ? `Server default — no workspace override set.\nProvider: ${config.provider}\nModel: ${config.model}${config.dimension ? `\nDimension: ${config.dimension}` : ''}`
    : `Workspace override.\nProvider: ${config.provider}\nModel: ${config.model}${config.dimension ? `\nDimension: ${config.dimension}` : ''}`;

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div
            className={`flex items-center gap-2 py-1.5 px-2.5 rounded-md border ${
              isServerDefault
                ? 'bg-muted/40 border-muted-foreground/10'
                : 'bg-primary/5 border-primary/20'
            } ${className ?? ''}`}
          >
            {/* Source icon */}
            {isServerDefault ? (
              <Server className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            ) : (
              <Sparkles className="h-3.5 w-3.5 shrink-0 text-primary" />
            )}

            {/* Label + values */}
            <div className="min-w-0 flex-1">
              <div className="text-[10px] uppercase tracking-wide text-muted-foreground font-medium leading-none mb-0.5">
                {label}
              </div>
              <div className="flex items-center gap-1.5 flex-wrap">
                <Badge
                  variant="secondary"
                  className="text-[11px] px-1.5 py-0 h-4 font-mono capitalize"
                >
                  {config.provider}
                </Badge>
                <span className="text-[11px] font-mono text-foreground truncate">
                  {config.model}
                </span>
                {config.dimension !== undefined && (
                  <span className="text-[10px] text-muted-foreground">
                    {config.dimension}d
                  </span>
                )}
              </div>
            </div>

            {/* Source label */}
            <span
              className={`text-[9px] uppercase tracking-wide shrink-0 font-semibold ${
                isServerDefault ? 'text-muted-foreground' : 'text-primary'
              }`}
            >
              {isServerDefault ? 'server' : 'workspace'}
            </span>
          </div>
        </TooltipTrigger>
        <TooltipContent side="right" className="max-w-xs whitespace-pre-line text-xs">
          {tooltipText}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

/**
 * Resolve the effective LLM config from workspace + server health.
 * Returns null when health is not yet loaded.
 */
export function resolveEffectiveLlmConfig(
  workspaceProvider?: string | null,
  workspaceModel?: string | null,
  healthLlm?: { name: string; model: string } | null,
): EffectiveProviderConfig | null {
  if (!healthLlm) return null;

  const hasWorkspaceOverride =
    workspaceProvider && workspaceProvider.trim() !== '' &&
    workspaceModel && workspaceModel.trim() !== '';

  if (hasWorkspaceOverride) {
    return {
      provider: workspaceProvider!,
      model: workspaceModel!,
      source: 'workspace',
    };
  }

  return {
    provider: healthLlm.name,
    model: healthLlm.model,
    source: 'server-default',
  };
}

/**
 * Resolve the effective embedding config from workspace + server health.
 */
export function resolveEffectiveEmbeddingConfig(
  workspaceProvider?: string | null,
  workspaceModel?: string | null,
  workspaceDimension?: number | null,
  healthEmbedding?: { name: string; model: string; dimension: number } | null,
): EffectiveProviderConfig | null {
  if (!healthEmbedding) return null;

  const hasWorkspaceOverride =
    workspaceProvider && workspaceProvider.trim() !== '' &&
    workspaceModel && workspaceModel.trim() !== '';

  if (hasWorkspaceOverride) {
    return {
      provider: workspaceProvider!,
      model: workspaceModel!,
      source: 'workspace',
      dimension: workspaceDimension ?? healthEmbedding.dimension,
    };
  }

  return {
    provider: healthEmbedding.name,
    model: healthEmbedding.model,
    source: 'server-default',
    dimension: healthEmbedding.dimension,
  };
}

/**
 * Resolve the effective vision LLM config from workspace + server health.
 *
 * WHY INVARIANT: A workspace vision_llm_model is only applied when the
 * workspace vision_llm_provider is ALSO set. An orphaned model (stored without
 * a provider) causes provider/model mismatch (e.g., gpt-4.1-nano on Ollama).
 * The backend (helpers.rs) enforces this invariant at task creation time —
 * we mirror the same logic in the UI so the displayed value matches reality.
 */
export function resolveEffectiveVisionConfig(
  workspaceProvider?: string | null,
  workspaceModel?: string | null,
  healthVision?: { name: string; default_model: string } | null,
): EffectiveProviderConfig | null {
  if (!healthVision) return null;

  // Mirror helpers.rs invariant: only apply workspace model if provider is ALSO set.
  const hasExplicitProvider = workspaceProvider && workspaceProvider.trim() !== '';
  const hasExplicitModel = workspaceModel && workspaceModel.trim() !== '';

  if (hasExplicitProvider && hasExplicitModel) {
    return {
      provider: workspaceProvider!,
      model: workspaceModel!,
      source: 'workspace',
    };
  }

  // Partial override or no override → server default
  return {
    provider: healthVision.name,
    model: healthVision.default_model,
    source: 'server-default',
  };
}
