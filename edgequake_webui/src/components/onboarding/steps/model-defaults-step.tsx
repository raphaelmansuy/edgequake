'use client';

import { CustomModelPickers } from '@/components/onboarding/steps/custom-model-pickers';
import { ImageEmbeddingProbeNotice } from '@/components/onboarding/steps/image-embedding-probe-notice';
import { ModelSourceChoice } from '@/components/onboarding/steps/model-source-choice';
import { DecisionEngineFields } from '@/components/workspace/decision-engine-fields';
import type { EmbeddingSelection } from '@/components/workspace/embedding-model-selector';
import type { LLMSelection } from '@/components/workspace/llm-model-selector';
import { useDecisionModels, useDecisionStatus } from '@/hooks/use-decision-status';
import { useInheritedModelDefaults } from '@/hooks/use-inherited-model-defaults';
import type { WizardEligibility } from '@/hooks/use-wizard-eligibility';
import { blockedRoles, type WizardRole } from '@/lib/onboarding/defaults-eligibility';
import { formatInheritModelLabel } from '@/lib/onboarding/inherited-defaults';
import type { WizardDraft } from '@/lib/onboarding/wizard-state';
import { draftIssue, type ExtractionModeDraft } from '@/lib/workspace/extraction-mode-draft';
import { ImageOff } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface ModelDefaultsStepProps {
  draft: WizardDraft;
  onChange: (patch: Partial<WizardDraft>) => void;
  llm?: LLMSelection;
  embedding?: EmbeddingSelection;
  vision?: LLMSelection;
  onLlmChange: (v: LLMSelection | undefined) => void;
  onEmbeddingChange: (v: EmbeddingSelection | undefined) => void;
  onVisionChange: (v: LLMSelection | undefined) => void;
  advancedOpen: boolean;
  onAdvancedOpenChange: (open: boolean) => void;
  /** When set, resolve LLM/Embed/Vision via tenant → server ladder (Create Workspace). */
  tenantId?: string | null;
  /**
   * Which eligible set the advanced pickers read.
   * First-run has no tenant, so the default is deployment until a tenant id exists.
   */
  choiceScope?: 'deployment' | 'tenant' | 'workspace';
  /** SPEC-179: verdicts on the pair this step will save. Absent on surfaces that do not judge yet. */
  eligibility?: WizardEligibility;
  /** Organization admins can open the offer from here. */
  canManageProviders?: boolean;
  /** SPEC-180: the draft is a normal workspace. It has no extraction engine to configure. */
  hidesExtractionEngine?: boolean;
  /** SPEC-180: the draft embeds images, so the server default (text only) cannot serve it. */
  requiresImageEmbedding?: boolean;
}

type Inherited = ReturnType<typeof useInheritedModelDefaults>;

/** Roles whose inherited default can seed a custom pick (set, and not refused by the offer). */
function seedableRoles(
  inherited: Inherited,
  eligibility?: WizardEligibility,
): Record<WizardRole, boolean> {
  // Wait until the offer has been judged. Seeding earlier copies a pair the workspace may refuse.
  if (eligibility?.isLoading) {
    return { llm: false, embedding: false, vision: false };
  }
  const refused = new Set(eligibility ? blockedRoles(eligibility.verdicts) : []);
  return {
    llm: Boolean(inherited.defaultLlmProvider && inherited.defaultLlmModel) && !refused.has('llm'),
    embedding:
      Boolean(inherited.defaultEmbeddingProvider && inherited.defaultEmbeddingModel) &&
      !refused.has('embedding'),
    vision:
      Boolean(inherited.defaultVisionProvider && inherited.defaultVisionModel) &&
      !refused.has('vision'),
  };
}

/**
 * SPEC-101 — Defaults-first model step (LAW-101-2 / LAW-101-3).
 * One choice (follow defaults / choose models); pickers mount only for the second.
 * Choosing models starts from the usable defaults, so the user changes only what they need.
 */
export function ModelDefaultsStep({
  draft,
  onChange,
  llm,
  embedding,
  vision,
  onLlmChange,
  onEmbeddingChange,
  onVisionChange,
  advancedOpen,
  onAdvancedOpenChange,
  tenantId,
  choiceScope,
  eligibility,
  canManageProviders = false,
  hidesExtractionEngine = false,
  requiresImageEmbedding = false,
}: ModelDefaultsStepProps) {
  const { t } = useTranslation();
  const pickerScope = choiceScope ?? (tenantId ? 'tenant' : 'deployment');
  const inherited = useInheritedModelDefaults(tenantId ?? null);
  const { isLoading, hasConfiguredDefaults } = inherited;
  const decisionStatus = useDecisionStatus(draft.decisionModel);
  const decisionModels = useDecisionModels();
  const decisionDraft: ExtractionModeDraft = {
    mode: 'inherit',
    enabled: draft.decisionEnabled,
    preset: draft.decisionPreset,
    model: draft.decisionModel,
    packSize: draft.decisionPackSize,
  };
  const decisionIssue = draftIssue(decisionDraft, decisionStatus.data?.limits);

  // Only force Advanced after defaults have finished loading and are incomplete.
  useEffect(() => {
    if (isLoading) return;
    if (!hasConfiguredDefaults && draft.useServerDefaults) {
      onChange({ useServerDefaults: false });
      onAdvancedOpenChange(true);
    }
  }, [isLoading, hasConfiguredDefaults, draft.useServerDefaults, onChange, onAdvancedOpenChange]);

  // Text and images: the inherited embedding model reads text only, so the user must pick one.
  useEffect(() => {
    if (!requiresImageEmbedding || !draft.useServerDefaults) return;
    onChange({ useServerDefaults: false });
    onAdvancedOpenChange(true);
  }, [requiresImageEmbedding, draft.useServerDefaults, onChange, onAdvancedOpenChange]);

  const showAdvanced = advancedOpen && !isLoading;
  const [focusRequest, setFocusRequest] = useState<{ role: WizardRole; nonce: number } | null>(null);

  const useDefaults = () => {
    onChange({ useServerDefaults: true });
    onLlmChange(undefined);
    onEmbeddingChange(undefined);
    onVisionChange(undefined);
    onAdvancedOpenChange(false);
  };

  const customize = () => {
    onChange({ useServerDefaults: false });
    onAdvancedOpenChange(true);
    const seed = seedableRoles(inherited, eligibility);
    if (seed.llm && !llm) {
      onLlmChange({
        provider: inherited.defaultLlmProvider!,
        model: inherited.defaultLlmModel!,
        fullId: `${inherited.defaultLlmProvider}/${inherited.defaultLlmModel}`,
      });
    }
    if (seed.embedding && !embedding && !requiresImageEmbedding) {
      onEmbeddingChange({
        provider: inherited.defaultEmbeddingProvider!,
        model: inherited.defaultEmbeddingModel!,
        dimension: inherited.defaultEmbeddingDimension ?? 0,
      });
    }
    if (seed.vision && !vision) {
      onVisionChange({
        provider: inherited.defaultVisionProvider!,
        model: inherited.defaultVisionModel!,
        fullId: `${inherited.defaultVisionProvider}/${inherited.defaultVisionModel}`,
      });
    }
  };

  const chooseAnother = (role: WizardRole) => {
    customize();
    setFocusRequest((current) => ({ role, nonce: (current?.nonce ?? 0) + 1 }));
  };

  const tenantPrefix = t('onboarding.tenantDefaultShort', 'Organization default');
  const serverPrefix = t('onboarding.serverDefaultShort', 'Server default');
  const inheritLabel = (provider?: string, model?: string) =>
    formatInheritModelLabel({ source: inherited.source, provider, model, tenantPrefix, serverPrefix });

  return (
    <div className="space-y-4" data-testid="wizard-step-models">
      <ModelSourceChoice
        inherited={inherited}
        followsDefaults={!showAdvanced}
        onUseDefaults={useDefaults}
        onCustomize={customize}
        eligibility={eligibility}
        canManageProviders={canManageProviders}
        offerScope={pickerScope}
        onChooseAnother={chooseAnother}
      />

      {showAdvanced ? (
        <CustomModelPickers
          llm={llm}
          embedding={embedding}
          vision={vision}
          onLlmChange={onLlmChange}
          onEmbeddingChange={onEmbeddingChange}
          onVisionChange={onVisionChange}
          pickerScope={pickerScope}
          inheritLabels={{
            llm: inheritLabel(inherited.defaultLlmProvider, inherited.defaultLlmModel),
            embedding: inheritLabel(
              inherited.defaultEmbeddingProvider,
              inherited.defaultEmbeddingModel,
            ),
            vision: inheritLabel(inherited.defaultVisionProvider, inherited.defaultVisionModel),
          }}
          inheritPairs={{
            llm: { provider: inherited.defaultLlmProvider, model: inherited.defaultLlmModel },
            embedding: {
              provider: inherited.defaultEmbeddingProvider,
              model: inherited.defaultEmbeddingModel,
            },
            vision: {
              provider: inherited.defaultVisionProvider,
              model: inherited.defaultVisionModel,
            },
          }}
          reasoningEffort={draft.reasoningEffort}
          onReasoningEffortChange={(reasoningEffort) => onChange({ reasoningEffort })}
          focusRole={focusRequest?.role}
          focusNonce={focusRequest?.nonce ?? 0}
          imageEmbeddingOnly={requiresImageEmbedding}
          embeddingNotice={
            /* SPEC-180 08: the reason and the deployment's answer sit with the pick they judge. */
            requiresImageEmbedding && !embedding?.provider ? (
              <p
                className="flex items-start gap-1.5 text-xs text-amber-700 dark:text-amber-400"
                data-testid="wizard-image-embedding-required"
                role="status"
              >
                <ImageOff aria-hidden className="mt-px h-3.5 w-3.5 shrink-0" />
                {t(
                  'ragProfile.evidence.textImageHint',
                  'Needs an embedding model that accepts images.',
                )}
              </p>
            ) : (
              <ImageEmbeddingProbeNotice
                enabled={requiresImageEmbedding}
                probe={eligibility?.imageProbe ?? { status: 'idle' }}
                pair={{
                  provider: embedding?.provider ?? inherited.defaultEmbeddingProvider,
                  model: embedding?.model ?? inherited.defaultEmbeddingModel,
                }}
              />
            )
          }
        />
      ) : null}

      {!isLoading && !hasConfiguredDefaults && !showAdvanced ? (
        <p
          className="text-sm text-amber-700 dark:text-amber-400"
          data-testid="wizard-models-required-hint"
        >
          {t(
            'onboarding.modelsRequiredHint',
            tenantId
              ? 'Organization and server defaults are incomplete. Choose models to continue.'
              : 'Server defaults are incomplete. Choose models to continue.',
          )}
        </p>
      ) : null}

      {hidesExtractionEngine ? null : (
      <section className="space-y-2" aria-labelledby="wizard-decision-heading">
        <h4
          id="wizard-decision-heading"
          className="text-xs font-semibold uppercase tracking-wide text-muted-foreground"
        >
          {t('onboarding.decisionSection', 'Extraction engine')}
        </h4>
        <div data-testid="wizard-decision-engine">
          <DecisionEngineFields
            draft={decisionDraft}
            onChange={(patch) =>
              onChange({
                ...(patch.enabled !== undefined ? { decisionEnabled: patch.enabled } : {}),
                ...(patch.model !== undefined ? { decisionModel: patch.model } : {}),
                ...(patch.preset !== undefined ? { decisionPreset: patch.preset } : {}),
                ...(patch.packSize !== undefined ? { decisionPackSize: patch.packSize } : {}),
              })
            }
            limits={decisionStatus.data?.limits}
            issue={decisionIssue}
            forcedOn={decisionStatus.data?.activation === 'forced'}
            lockedByAdmin={decisionStatus.data?.activation === 'locked'}
            models={decisionModels.data?.models}
            modelsLoading={decisionModels.isLoading}
            host={
              decisionStatus.data?.provider?.base_url_host ??
              decisionStatus.data?.backend?.base_url_host ??
              null
            }
            status={decisionStatus.data}
            statusLoading={decisionStatus.isLoading}
            statusError={decisionStatus.isError}
            onRetryStatus={() => void decisionStatus.refetch()}
            statusRetrying={decisionStatus.isFetching}
          />
        </div>
      </section>
      )}
    </div>
  );
}
