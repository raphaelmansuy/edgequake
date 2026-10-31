'use client';

import { ReasoningEffortSelect } from '@/components/settings/reasoning-effort-select';
import { Label } from '@/components/ui/label';
import {
  EmbeddingModelSelector,
  type EmbeddingSelection,
} from '@/components/workspace/embedding-model-selector';
import { LLMModelSelector, type LLMSelection } from '@/components/workspace/llm-model-selector';
import { useLlmCapabilityModels } from '@/hooks/use-llm-capability-models';
import type { WizardRole } from '@/lib/onboarding/defaults-eligibility';
import { openPickerTrigger, revealWizardRolePicker } from '@/lib/onboarding/wizard-role-picker';
import {
  effectiveEffortWhenAuto,
  modelSupportsThinking,
  supportedReasoningEffortsForModel,
} from '@/lib/settings/reasoning-effort-supported';
import { useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';

/** Provider and model side by side on wide forms; the “use default” link spans both columns. */
const SIDE_BY_SIDE = 'md:grid md:grid-cols-2 md:gap-x-3 md:space-y-0 md:[&>button]:col-span-2 md:[&>button]:justify-self-start';

export interface CustomModelPickersProps {
  llm?: LLMSelection;
  embedding?: EmbeddingSelection;
  vision?: LLMSelection;
  onLlmChange: (v: LLMSelection | undefined) => void;
  onEmbeddingChange: (v: EmbeddingSelection | undefined) => void;
  onVisionChange: (v: LLMSelection | undefined) => void;
  pickerScope: 'deployment' | 'tenant' | 'workspace';
  inheritLabels: { llm: string; embedding: string; vision: string };
  /** Inherited pairs. Each "use default" link hides when that pair is not in this scope's offer. */
  inheritPairs?: {
    llm?: { provider?: string; model?: string };
    embedding?: { provider?: string; model?: string };
    vision?: { provider?: string; model?: string };
  };
  reasoningEffort?: string;
  onReasoningEffortChange: (value: string | undefined) => void;
  /** Set by "Choose another" so that role's provider menu opens, even if custom mode is already on. */
  focusRole?: WizardRole;
  focusNonce?: number;
  /** SPEC-180: the workspace embeds images, so the embedding list shows image-capable models. */
  imageEmbeddingOnly?: boolean;
  /**
   * SPEC-180 08: the image embedding probe, rendered under the embedding picker so the
   * reason appears where the choice is made — not only in the footer.
   */
  embeddingNotice?: React.ReactNode;
}

/** Two-step provider → model pickers for LLM, embedding and vision, plus reasoning effort. */
export function CustomModelPickers({
  llm,
  embedding,
  vision,
  onLlmChange,
  onEmbeddingChange,
  onVisionChange,
  pickerScope,
  inheritLabels,
  inheritPairs,
  reasoningEffort,
  onReasoningEffortChange,
  focusRole,
  focusNonce = 0,
  imageEmbeddingOnly = false,
  embeddingNotice,
}: CustomModelPickersProps) {
  const { t } = useTranslation();
  const llmModels = useLlmCapabilityModels();
  useEffect(() => {
    if (!focusRole || focusNonce === 0) return;
    let cancelled = false;
    let attempts = 0;
    const reveal = () => {
      if (cancelled) return;
      const trigger = revealWizardRolePicker(focusRole);
      if (!trigger) {
        if (attempts < 24) {
          attempts += 1;
          window.setTimeout(reveal, 50);
        }
        return;
      }
      // Let the dialog body finish scrolling before the menu anchors to the trigger.
      window.setTimeout(() => {
        if (!cancelled) openPickerTrigger(trigger);
      }, 60);
    };
    reveal();
    return () => {
      cancelled = true;
    };
  }, [focusRole, focusNonce]);

  const reasoning = useMemo(
    () => ({
      supported: supportedReasoningEffortsForModel(llmModels, llm?.provider, llm?.model),
      thinking: modelSupportsThinking(llmModels, llm?.provider, llm?.model),
      auto: effectiveEffortWhenAuto(llmModels, llm?.provider, llm?.model, 'fleet'),
    }),
    [llmModels, llm?.provider, llm?.model],
  );

  return (
    <div className="min-w-0 space-y-4 rounded-xl border bg-card p-3.5" data-testid="wizard-models-advanced">
      <p className="text-xs text-muted-foreground">
        {t('onboarding.customModelsHint', 'Choose a provider, then a model.')}
      </p>
      <div className="grid min-w-0 gap-4">
        <div className="grid min-w-0 gap-1.5" data-wizard-model-role="llm">
          <Label>{t('workspace.llmModel', 'LLM Model')}</Label>
          <LLMModelSelector
            value={llm}
            onChange={onLlmChange}
            showCapabilityFilters={false}
            showUsageHint
            panelClassName={SIDE_BY_SIDE}
            inheritLabel={inheritLabels.llm}
            inheritPair={inheritPairs?.llm}
            choiceScope={pickerScope}
          />
        </div>
        <div className="grid min-w-0 gap-1.5" data-wizard-model-role="embedding">
          <Label>{t('workspace.embeddingModel', 'Embedding Model')}</Label>
          <EmbeddingModelSelector
            value={embedding}
            onChange={onEmbeddingChange}
            panelClassName={SIDE_BY_SIDE}
            inheritLabel={inheritLabels.embedding}
            inheritPair={inheritPairs?.embedding}
            choiceScope={pickerScope}
            imageOnly={imageEmbeddingOnly}
          />
          {/* SPEC-180 08: the deployment's answer sits with the pick it judges. */}
          {embeddingNotice}
        </div>
        <div className="grid min-w-0 gap-1.5" data-wizard-model-role="vision">
          <Label>{t('workspace.visionLLM', 'Vision LLM')}</Label>
          <LLMModelSelector
            value={vision}
            onChange={onVisionChange}
            filterVision
            showCapabilityFilters={false}
            showUsageHint
            choiceScope={pickerScope}
            usage="vlm"
            panelClassName={SIDE_BY_SIDE}
            inheritLabel={inheritLabels.vision}
            inheritPair={inheritPairs?.vision}
          />
        </div>
        <ReasoningEffortSelect
          value={reasoningEffort}
          onChange={onReasoningEffortChange}
          supported={reasoning.supported}
          thinkingSupported={reasoning.thinking}
          effectiveWhenAuto={reasoning.auto}
          label={t('onboarding.reasoningEffort', 'Default reasoning effort')}
          data-testid="wizard-reasoning-effort"
        />
      </div>
    </div>
  );
}
