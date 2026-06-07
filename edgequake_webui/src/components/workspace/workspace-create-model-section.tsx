'use client';

import { Button } from '@/components/ui/button';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { Label } from '@/components/ui/label';
import { useServerModelDefaults } from '@/hooks/use-server-model-defaults';
import { cn } from '@/lib/utils';
import { ChevronDown } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  EmbeddingModelSelector,
  type EmbeddingSelection,
} from './embedding-model-selector';
import { LLMModelSelector, type LLMSelection } from './llm-model-selector';

export interface WorkspaceCreateModelSectionProps {
  llm: LLMSelection | undefined;
  embedding: EmbeddingSelection | undefined;
  vision: LLMSelection | undefined;
  onLlmChange: (v: LLMSelection | undefined) => void;
  onEmbeddingChange: (v: EmbeddingSelection | undefined) => void;
  onVisionChange: (v: LLMSelection | undefined) => void;
  /** When true, parent should omit model fields from create payload (use server defaults). */
  onUseServerDefaultsChange?: (useDefaults: boolean) => void;
}

/**
 * Collapsible model configuration for workspace creation.
 * Hides advanced selectors when the server already has defaults (GitHub #233).
 */
export function WorkspaceCreateModelSection({
  llm,
  embedding,
  vision,
  onLlmChange,
  onEmbeddingChange,
  onVisionChange,
  onUseServerDefaultsChange,
}: WorkspaceCreateModelSectionProps) {
  const { t } = useTranslation();
  const { hasConfiguredDefaults, defaultLlmModel, defaultLlmProvider, defaultEmbeddingModel, defaultEmbeddingProvider, isLoading } =
    useServerModelDefaults();

  const [advancedOpen, setAdvancedOpen] = useState(!hasConfiguredDefaults);

  useEffect(() => {
    if (!isLoading) {
      setAdvancedOpen(!hasConfiguredDefaults);
    }
  }, [hasConfiguredDefaults, isLoading]);

  const useServerDefaults = hasConfiguredDefaults && !advancedOpen;

  useEffect(() => {
    onUseServerDefaultsChange?.(useServerDefaults);
  }, [useServerDefaults, onUseServerDefaultsChange]);

  if (isLoading) {
    return null;
  }

  return (
    <div className="rounded-lg border p-3 space-y-3" data-testid="workspace-create-model-section">
      {hasConfiguredDefaults && !advancedOpen ? (
        <div className="space-y-1" data-testid="workspace-create-server-defaults-summary">
          <p className="text-sm font-medium">{t('workspace.usingServerDefaults', 'Using server defaults')}</p>
          <p className="text-xs text-muted-foreground font-mono">
            LLM: {defaultLlmProvider}/{defaultLlmModel} · Embedding: {defaultEmbeddingProvider}/
            {defaultEmbeddingModel}
          </p>
          <p className="text-xs text-muted-foreground">
            {t(
              'workspace.serverDefaultsHint',
              'Models are configured via environment variables. Expand advanced settings to override.'
            )}
          </p>
        </div>
      ) : null}

      <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
        <CollapsibleTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            className="w-full justify-between px-0 h-auto"
            data-testid="workspace-create-advanced-models-toggle"
          >
            <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              {hasConfiguredDefaults
                ? t('workspace.advancedModelConfig', 'Advanced model configuration')
                : t('workspace.modelsSection', 'Model Configuration')}
            </span>
            <ChevronDown className={cn('h-4 w-4 transition-transform', advancedOpen && 'rotate-180')} />
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent className="pt-3 space-y-3">
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="grid gap-2">
              <Label>
                {t('workspace.llmModel', 'LLM Model')}
                <span className="text-destructive ml-0.5">*</span>
              </Label>
              <LLMModelSelector value={llm} onChange={onLlmChange} />
            </div>
            <div className="grid gap-2">
              <Label>
                {t('workspace.embeddingModel', 'Embedding Model')}
                <span className="text-destructive ml-0.5">*</span>
              </Label>
              <EmbeddingModelSelector value={embedding} onChange={onEmbeddingChange} />
            </div>
            <div className="grid gap-2 sm:col-span-2">
              <Label>
                {t('workspace.visionLLM', 'Vision LLM')}
                <span className="text-destructive ml-0.5">*</span>
              </Label>
              <LLMModelSelector value={vision} onChange={onVisionChange} filterVision showUsageHint={false} />
            </div>
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}
