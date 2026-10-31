'use client';

import { Button } from '@/components/ui/button';
import { requestCreateWorkspace } from '@/lib/workspace/create-workspace-event';
import { Network } from 'lucide-react';
import Link from 'next/link';
import { useTranslation } from 'react-i18next';

/**
 * SPEC-180 — what a graph page says on a workspace that has no knowledge graph.
 * It explains, and offers the two useful next steps. No empty canvas, no spinner.
 */
export function NoGraphState({ title }: { title?: string }) {
  const { t } = useTranslation();
  return (
    <div
      className="flex h-full min-h-[24rem] flex-col items-center justify-center px-6 py-12 text-center"
      data-testid="no-graph-state"
    >
      <div className="mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-muted">
        <Network className="h-7 w-7 text-muted-foreground" aria-hidden="true" />
      </div>
      <h1 className="text-lg font-semibold tracking-tight">
        {title ?? t('ragProfile.noGraph.title', 'This workspace has no knowledge graph.')}
      </h1>
      <p className="mt-2 max-w-sm text-sm text-muted-foreground">
        {t('ragProfile.noGraph.body', 'It retrieves by embeddings and keywords.')}
      </p>
      <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
        <Button asChild data-testid="no-graph-documents">
          <Link href="/documents">{t('ragProfile.noGraph.documents', 'Go to documents')}</Link>
        </Button>
        <Button variant="outline" onClick={() => requestCreateWorkspace()} data-testid="no-graph-new">
          {t('ragProfile.noGraph.newGraph', 'New graph workspace')}
        </Button>
      </div>
    </div>
  );
}
