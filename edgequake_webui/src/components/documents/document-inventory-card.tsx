'use client';

/**
 * SPEC-146: narrow-viewport document inventory cards (< lg / 1024px).
 * Title, Status, Class, Share, Owner, actions — fold Entities/Created under More.
 * Table crush at tablet-768 is avoided by showing cards until lg.
 */

import { Checkbox } from '@/components/ui/checkbox';
import { ClassificationChip } from '@/components/security/classification-chip';
import { ShareModeBadge } from '@/components/security/share-mode-badge';
import { principalDisplayName } from '@/components/security/principal-select';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { DocumentActionsMenu } from '@/components/documents/document-actions-menu';
import { EnhancedStatusBadge } from '@/components/documents/enhanced-status-badge';
import { QuickActionButtons } from '@/components/documents/quick-action-buttons';
import type { Document } from '@/types';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface DocumentInventoryCardProps {
  doc: Document;
  isSelected: boolean;
  isActive: boolean;
  showAbacColumns: boolean;
  ownerNames: Record<string, string>;
  onSelect: (docId: string, checked: boolean) => void;
  onClick: (doc: Document) => void;
  onViewDetails: (doc: Document) => void;
  onViewInGraph: (doc: Document) => void;
  onViewPdf: (doc: Document) => void;
  onRetry: (id: string) => void;
  onRetryLabels?: (doc: Document) => void;
  onReprocess: (id: string) => void;
  onCancel: (trackId: string) => void;
  onDelete: (id: string) => void;
  isRetrying: boolean;
  isCancelling: boolean;
}

export function DocumentInventoryCard({
  doc,
  isSelected,
  isActive,
  showAbacColumns,
  ownerNames,
  onSelect,
  onClick,
  onViewDetails,
  onViewInGraph,
  onViewPdf,
  onRetry,
  onRetryLabels,
  onReprocess,
  onCancel,
  onDelete,
  isRetrying,
  isCancelling,
}: DocumentInventoryCardProps) {
  const { t } = useTranslation();
  const [more, setMore] = useState(false);
  const title = doc.title || doc.file_name || doc.id;
  const ownerLabel =
    (doc.owner_principal_id && ownerNames[doc.owner_principal_id]) ||
    (doc.owner_principal_id
      ? principalDisplayName(undefined, doc.owner_principal_id)
      : '—');

  return (
    <article
      className={`rounded-lg border bg-background p-3 shadow-sm ${
        isActive ? 'ring-2 ring-primary/40' : ''
      }`}
      data-testid="spec146-doc-card"
      onClick={() => onClick(doc)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onClick(doc);
        }
      }}
      role="button"
      tabIndex={0}
    >
      <div className="flex items-start gap-2">
        <Checkbox
          checked={isSelected}
          onCheckedChange={(c) => {
            onSelect(doc.id, !!c);
          }}
          onClick={(e) => e.stopPropagation()}
          aria-label={t('documents.bulk.selectOne', 'Select document')}
          className="mt-1"
        />
        <div className="min-w-0 flex-1 space-y-2">
          <div className="flex items-start justify-between gap-2">
            <h3 className="truncate font-medium text-sm text-foreground">{title}</h3>
            <div
              className="flex items-center gap-0.5 shrink-0"
              onClick={(e) => e.stopPropagation()}
            >
              <QuickActionButtons
                doc={doc}
                onViewDetails={onViewDetails}
                onPreview={onClick}
                onViewInGraph={onViewInGraph}
                onRetry={onRetry}
                onRetryLabels={onRetryLabels}
                isRetrying={isRetrying}
              >
                <DocumentActionsMenu
                  doc={doc}
                  onViewPdf={onViewPdf}
                  onCancel={onCancel}
                  onReprocess={onReprocess}
                  onDelete={onDelete}
                  isCancelling={isCancelling}
                />
              </QuickActionButtons>
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-1.5">
            <EnhancedStatusBadge document={doc} />
            {showAbacColumns ? (
              <>
                <ClassificationChip value={doc.classification} />
                <ShareModeBadge value={doc.share_mode} />
                {doc.security_status === 'quarantined' ? (
                  <Badge
                    variant="outline"
                    className="text-[10px] px-1 py-0 border-amber-500 text-amber-800 whitespace-nowrap shrink-0"
                    data-testid="spec146-quarantine-chip"
                  >
                    {t('security.quarantined', 'Labeling failed / Quarantined')}
                  </Badge>
                ) : null}
                <span className="text-xs text-muted-foreground whitespace-nowrap">
                  {t('documents.table.owner', 'Owner')}: {ownerLabel}
                </span>
              </>
            ) : null}
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-7 px-1 text-xs text-muted-foreground"
            onClick={(e) => {
              e.stopPropagation();
              setMore((v) => !v);
            }}
          >
            {more ? (
              <ChevronDown className="h-3.5 w-3.5 mr-1" />
            ) : (
              <ChevronRight className="h-3.5 w-3.5 mr-1" />
            )}
            {t('documents.card.more', 'More')}
          </Button>
          {more ? (
            <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-xs text-muted-foreground">
              <dt>{t('documents.table.entities', 'Entities')}</dt>
              <dd className="tabular-nums">{doc.entity_count ?? 0}</dd>
              <dt>{t('documents.table.created', 'Created')}</dt>
              <dd className="truncate">{doc.created_at ?? '—'}</dd>
              <dt>{t('documents.table.updated', 'Last Updated')}</dt>
              <dd className="truncate">{doc.updated_at ?? '—'}</dd>
            </dl>
          ) : null}
        </div>
      </div>
    </article>
  );
}
