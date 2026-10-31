'use client';

import { useWizardEligibility } from '@/hooks/use-wizard-eligibility';
import type { WizardEligibilityInput } from '@/hooks/use-wizard-eligibility';
import { hasAnyModelPick } from '@/lib/onboarding/wizard-picks';
import {
  describeBlocker,
  type BlockerTranslate,
} from '@/lib/onboarding/wizard-blocker-message';
import {
  wizardBlocker,
  type WizardDraft,
  type WizardStepId,
} from '@/lib/onboarding/wizard-state';
import { useTranslation } from 'react-i18next';

export interface WizardGateInput extends Omit<WizardEligibilityInput, 'useDefaults'> {
  stepId: WizardStepId;
  draft: WizardDraft;
  hasConfiguredDefaults: boolean;
  /** Reconfigure: Apply needs at least one change. */
  hasConfigChanges?: boolean;
  /** SPEC-180: false when the draft embeds images and the embedding pick cannot read them. */
  imageEmbeddingOk?: boolean;
}

/**
 * One answer to “may the user continue, and if not, why?” for a wizard step.
 * Create and Reconfigure share it, so the rule and the sentence cannot drift.
 */
export function useWizardGate(input: WizardGateInput) {
  const { t } = useTranslation();
  const { stepId, draft, hasConfiguredDefaults, hasConfigChanges, imageEmbeddingOk } = input;
  const eligibility = useWizardEligibility({ ...input, useDefaults: draft.useServerDefaults });
  const anyPick = hasAnyModelPick({
    llm: input.llm,
    embedding: input.embedding,
    vision: input.vision,
  });
  // A pair the deployment refuses to embed images with is as blocking as an ineligible pair.
  // The server's probe is the SSOT; the wizard must not walk past it to Review.
  const imageProbeFailed = eligibility.imageProbe.status === 'failed';
  const imageProbeChecking = eligibility.imageProbe.status === 'checking';
  // `eligibilityOk` stays about the offer and the pair; the probe has its own code so the
  // user is told the real reason instead of a generic "replace the model".
  const eligibilityOk = !eligibility.blocked && !eligibility.nothingOffered;
  const code = wizardBlocker(stepId, draft, {
    hasConfiguredDefaults,
    advancedModelsValid: anyPick,
    eligibilityOk,
    hasConfigChanges,
    imageEmbeddingOk,
    imageProbeFailed,
    imageProbeChecking,
  });
  const blockedReason = describeBlocker(code, t as BlockerTranslate, {
    verdicts: eligibility.verdicts,
    nothingOffered: eligibility.nothingOffered,
    scope: input.scope,
    imageProbeMessage: eligibility.imageProbe.message,
  });
  return { eligibility, eligibilityOk, canGoNext: code === null, blockedReason };
}
