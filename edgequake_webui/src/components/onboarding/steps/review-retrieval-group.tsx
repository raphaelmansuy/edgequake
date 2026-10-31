'use client';

import { Group, Row } from '@/components/onboarding/steps/review-parts';
import type { WizardDraft, WizardStepId } from '@/lib/onboarding/wizard-state';
import { useTranslation } from 'react-i18next';

export interface ReviewRetrievalGroupProps {
  draft: WizardDraft;
  onEditStep?: (step: WizardStepId) => void;
  editLabel: string;
}

/** SPEC-180 — the retrieval choice as the review step states it, with its permanence. */
export function ReviewRetrievalGroup({ draft, onEditStep, editLabel }: ReviewRetrievalGroupProps) {
  const { t } = useTranslation();
  const normal = draft.ragProfile === 'normal';
  return (
    <Group
      title={t('ragProfile.legend', 'Retrieval type')}
      onEdit={onEditStep ? () => onEditStep('retrieval') : undefined}
      editLabel={editLabel}
      testId="wizard-review-retrieval"
    >
      <Row
        label={t('ragProfile.review.retrieval', 'Retrieval')}
        value={t(`ragProfile.${draft.ragProfile}.title`, normal ? 'Normal' : 'Graph')}
      />
      <Row
        label={t('ragProfile.evidence.legend', 'Evidence')}
        value={
          draft.embeddingModalities === 'text,image'
            ? t('ragProfile.evidence.textImage', 'Text and images')
            : t('ragProfile.evidence.text', 'Text')
        }
      />
      {normal ? (
        <>
          <Row
            label={t('ragProfile.review.extraction', 'Extraction model')}
            value={t('ragProfile.review.off', 'Off')}
          />
          <Row
            label={t('ragProfile.review.keywordIndex', 'Keyword index')}
            value={t('ragProfile.review.keywordIndexOn', 'On, {{language}}', {
              language: t(`ragProfile.language.${draft.ftsRegconfig}`, draft.ftsRegconfig),
            })}
          />
        </>
      ) : null}
      <p
        className="pt-1 text-xs text-muted-foreground"
        data-testid="wizard-review-permanent"
      >
        {t(
          'ragProfile.review.permanent',
          'This choice is permanent. To use the other type later, create a new workspace and upload again.',
        )}
      </p>
    </Group>
  );
}
