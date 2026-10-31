'use client';

/**
 * SPEC-180 — the retrieval step: Graph or Normal, text or text and images, keyword language.
 *
 * Presentational. The rules (what a choice resets) live in `retrieval-draft.ts`.
 */

import { OptionCard } from '@/components/onboarding/option-card';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  FTS_REGCONFIGS,
  parseFtsRegconfig,
  type EmbeddingModalities,
  type RagProfile,
} from '@/constants/rag-profile';
import { patchForModalities, patchForProfile } from '@/lib/onboarding/retrieval-draft';
import type { WizardDraft } from '@/lib/onboarding/wizard-state';
import { Network, Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface WorkspaceRetrievalStepProps {
  draft: WizardDraft;
  onChange: (patch: Partial<WizardDraft>) => void;
}

interface ProfileOption {
  value: RagProfile;
  icon: typeof Network;
  testId: string;
}

const PROFILE_OPTIONS: ProfileOption[] = [
  { value: 'graph', icon: Network, testId: 'wizard-profile-graph' },
  { value: 'normal', icon: Search, testId: 'wizard-profile-normal' },
];

function ProfileCard({
  option,
  selected,
  onSelect,
}: {
  option: ProfileOption;
  selected: boolean;
  onSelect: () => void;
}) {
  const { t } = useTranslation();
  const Icon = option.icon;
  const key = `ragProfile.${option.value}`;
  const defaults = {
    graph: {
      title: 'Graph',
      summary: 'Entities, relationships, and communities.',
      detail: 'Higher ingest cost. Better for multi-hop questions.',
    },
    normal: {
      title: 'Normal',
      summary: 'Embeddings and keyword match. No knowledge graph.',
      detail: 'Lower ingest cost. Enough for lookup.',
    },
  }[option.value];
  return (
    <OptionCard
      selected={selected}
      onSelect={onSelect}
      title={t(`${key}.title`, defaults.title)}
      data-testid={option.testId}
    >
      <div className="flex items-start gap-2.5 pl-6">
        <Icon aria-hidden className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
        <div className="min-w-0 space-y-0.5 text-xs leading-relaxed text-muted-foreground">
          <p className="text-foreground/90">{t(`${key}.summary`, defaults.summary)}</p>
          <p>{t(`${key}.detail`, defaults.detail)}</p>
        </div>
      </div>
    </OptionCard>
  );
}

function EvidenceChoice({
  draft,
  onChange,
}: WorkspaceRetrievalStepProps) {
  const { t } = useTranslation();
  const imagesAllowed = draft.ragProfile === 'normal';
  const pick = (modalities: EmbeddingModalities) =>
    onChange(patchForModalities(draft, modalities));
  return (
    <fieldset className="space-y-2" data-testid="wizard-evidence">
      <legend className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {t('ragProfile.evidence.legend', 'Evidence')}
      </legend>
      <div role="radiogroup" className="grid gap-2 md:grid-cols-2">
        <OptionCard
          selected={draft.embeddingModalities === 'text'}
          onSelect={() => pick('text')}
          title={t('ragProfile.evidence.text', 'Text')}
          data-testid="wizard-evidence-text"
        />
        <OptionCard
          selected={draft.embeddingModalities === 'text,image'}
          onSelect={() => pick('text,image')}
          disabled={!imagesAllowed}
          title={t('ragProfile.evidence.textImage', 'Text and images')}
          data-testid="wizard-evidence-text-image"
        >
          <p className="pl-6 text-xs text-muted-foreground">
            {imagesAllowed
              ? t(
                  'ragProfile.evidence.textImageHint',
                  'Needs an embedding model that accepts images.',
                )
              : t(
                  'ragProfile.evidence.textImageNeedsNormal',
                  'Available with the Normal retrieval type.',
                )}
          </p>
        </OptionCard>
      </div>
      {/* Escape hatch: an image workspace whose deployment offers no image model is not stuck. */}
      {imagesAllowed && draft.embeddingModalities === 'text,image' ? (
        <p className="text-xs text-muted-foreground" data-testid="wizard-evidence-fallback">
          {t('ragProfile.evidence.noImageModelHint', 'No image model available?')}{' '}
          <button
            type="button"
            className="font-medium text-primary underline-offset-2 hover:underline"
            onClick={() => pick('text')}
            data-testid="wizard-evidence-use-text"
          >
            {t('ragProfile.evidence.useTextInstead', 'Use Text only')}
          </button>
        </p>
      ) : null}
    </fieldset>
  );
}

function KeywordLanguage({ draft, onChange }: WorkspaceRetrievalStepProps) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-1.5 md:max-w-xs" data-testid="wizard-fts-language">
      <Label htmlFor="wizard-fts-regconfig">
        {t('ragProfile.keywordLanguage', 'Keyword language')}
      </Label>
      <Select
        value={draft.ftsRegconfig}
        onValueChange={(value) => onChange({ ftsRegconfig: parseFtsRegconfig(value) })}
      >
        <SelectTrigger id="wizard-fts-regconfig" data-testid="wizard-fts-regconfig">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {FTS_REGCONFIGS.map((language) => (
            <SelectItem key={language} value={language}>
              {t(`ragProfile.language.${language}`, language)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <p className="text-xs text-muted-foreground">
        {t('ragProfile.keywordLanguageHint', 'Used by the keyword index to split and match words.')}
      </p>
    </div>
  );
}

export function WorkspaceRetrievalStep({ draft, onChange }: WorkspaceRetrievalStepProps) {
  const { t } = useTranslation();
  return (
    <div className="space-y-5" data-testid="wizard-step-retrieval">
      <fieldset className="space-y-2">
        <legend className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          {t('ragProfile.legend', 'Retrieval type')}
        </legend>
        <div
          role="radiogroup"
          aria-label={t('ragProfile.legend', 'Retrieval type')}
          className="grid gap-3 md:grid-cols-2"
        >
          {PROFILE_OPTIONS.map((option) => (
            <ProfileCard
              key={option.value}
              option={option}
              selected={draft.ragProfile === option.value}
              onSelect={() => onChange(patchForProfile(draft, option.value))}
            />
          ))}
        </div>
      </fieldset>

      <EvidenceChoice draft={draft} onChange={onChange} />
      <KeywordLanguage draft={draft} onChange={onChange} />

      <p className="rounded-lg border border-dashed bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
        {t(
          'ragProfile.permanent',
          'This choice is permanent. To use the other type later, create a new workspace.',
        )}
      </p>
    </div>
  );
}
