/**
 * @module WorkspaceRetrievalCard
 * @description Read-only retrieval type of a workspace (SPEC-180).
 *
 * The type, evidence and keyword language are fixed at create, so the card has no control.
 * It reads like text, not like a disabled form.
 */
'use client';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { parseFtsRegconfig, parseRagProfile, capabilitiesOf } from '@/constants/rag-profile';
import { Route } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { Workspace } from '@/types';

type Row = { key: string; label: string; value: string };

export function WorkspaceRetrievalCard({ workspace }: { workspace: Workspace }) {
  const { t } = useTranslation();
  const caps = capabilitiesOf(workspace);
  const profile = parseRagProfile(workspace.rag_profile);
  const language = parseFtsRegconfig(workspace.fts_regconfig);

  const rows: Row[] = [
    {
      key: 'type',
      label: t('ragProfile.settings.type', 'Type'),
      value: t(`ragProfile.${profile}.title`, profile === 'normal' ? 'Normal' : 'Graph'),
    },
    {
      key: 'evidence',
      label: t('ragProfile.evidence.legend', 'Evidence'),
      value: caps.embeds_images
        ? t('ragProfile.evidence.textImage', 'Text and images')
        : t('ragProfile.evidence.text', 'Text'),
    },
  ];
  if (!caps.extracts_graph) {
    rows.push({
      key: 'language',
      label: t('ragProfile.keywordLanguage', 'Keyword language'),
      value: t(`ragProfile.language.${language}`, language),
    });
  }

  return (
    <Card className="gap-2 py-4" data-testid="workspace-retrieval-card">
      <CardHeader className="px-4">
        <CardTitle className="flex items-center gap-2 text-base">
          <Route className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          {t('ragProfile.settings.title', 'Retrieval')}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3 px-4">
        <dl className="grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1.5 text-sm">
          {rows.map((row) => (
            <div key={row.key} className="contents" data-testid={`workspace-retrieval-${row.key}`}>
              <dt className="text-muted-foreground">{row.label}</dt>
              <dd className="font-medium">{row.value}</dd>
            </div>
          ))}
        </dl>
        <p className="text-xs text-muted-foreground" data-testid="workspace-retrieval-permanent">
          {caps.extracts_graph
            ? t(
                'ragProfile.settings.permanentGraph',
                'Permanent. Create a new workspace to use Normal retrieval.',
              )
            : t(
                'ragProfile.settings.permanentNormal',
                'Permanent. Create a new workspace to use Graph retrieval.',
              )}
        </p>
      </CardContent>
    </Card>
  );
}
