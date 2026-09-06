'use client';

import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import {
  nextDocumentSortState,
  type SortDirection,
  type SortField,
} from '@/lib/documents/document-sort';
import type { DocStatus } from '@/hooks/use-document-preferences';
import { ArrowDown, ArrowUp } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import {
  CLASSIFICATIONS,
  type Classification,
} from '@/lib/security/security-fields';

export type { DocStatus, SortDirection, SortField };

export type ClassificationFilter = 'all' | Classification;

interface DocumentFiltersProps {
  status: DocStatus;
  sortField: SortField;
  sortDirection: SortDirection;
  onStatusChange: (status: DocStatus) => void;
  onSortFieldChange: (field: SortField) => void;
  onSortDirectionChange: (direction: SortDirection) => void;
  statusCounts?: Record<DocStatus, number>;
  classification?: ClassificationFilter;
  onClassificationChange?: (value: ClassificationFilter) => void;
  showClassificationFilter?: boolean;
}

export function DocumentFilters({
  status,
  sortField,
  sortDirection,
  onStatusChange,
  onSortFieldChange,
  onSortDirectionChange,
  statusCounts,
  classification = 'all',
  onClassificationChange,
  showClassificationFilter = false,
}: DocumentFiltersProps) {
  const { t } = useTranslation();

  const toggleSort = (field: SortField) => {
    const next = nextDocumentSortState(sortField, sortDirection, field);
    onSortFieldChange(next.field);
    onSortDirectionChange(next.direction);
  };

  const getStatusLabel = (statusKey: DocStatus) => {
    const label = t(`documents.status.${statusKey}`);
    if (statusCounts && statusCounts[statusKey] !== undefined) {
      return `${label} (${statusCounts[statusKey]})`;
    }
    return label;
  };

  return (
    <div
      className="flex flex-wrap items-center gap-3 min-w-0 max-w-full"
      data-testid="document-filters"
    >
      {/* Status Filter */}
      <div className="space-y-1 min-w-0">
        <Label htmlFor="doc-filter-status" className="text-xs text-muted-foreground">
          {t('documents.filter.status')}
        </Label>
        <Select
          value={status}
          onValueChange={(v) => onStatusChange(v as DocStatus)}
        >
          <SelectTrigger id="doc-filter-status" className="w-40 max-w-full h-10">
            <SelectValue placeholder={t('documents.filter.status')} />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{getStatusLabel('all')}</SelectItem>
            <SelectItem value="pending">{getStatusLabel('pending')}</SelectItem>
            <SelectItem value="processing">{getStatusLabel('processing')}</SelectItem>
            <SelectItem value="completed">{getStatusLabel('completed')}</SelectItem>
            <SelectItem value="failed">{getStatusLabel('failed')}</SelectItem>
            <SelectItem value="partial_failure">{getStatusLabel('partial_failure')}</SelectItem>
            <SelectItem value="cancelled">{getStatusLabel('cancelled')}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {showClassificationFilter ? (
        <div className="space-y-1 min-w-0">
          <Label
            htmlFor="doc-filter-classification"
            className="text-xs text-muted-foreground"
          >
            {t('documents.filter.classification', 'Classification')}
          </Label>
          <Select
            value={classification}
            onValueChange={(v) =>
              onClassificationChange?.(v as ClassificationFilter)
            }
          >
            <SelectTrigger
              id="doc-filter-classification"
              className="w-44 max-w-full h-10"
              data-testid="spec146-classification-filter"
            >
              <SelectValue
                placeholder={t('documents.filter.classification', 'Classification')}
              />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">
                {t('documents.filter.allClassifications', 'All classifications')}
              </SelectItem>
              {CLASSIFICATIONS.map((c) => (
                <SelectItem key={c.value} value={c.value}>
                  {t(`security.classification.${c.value}`, c.label)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      ) : null}

      {/* Divider */}
      <div className="h-6 w-px bg-border hidden sm:block" />

      {/* Sort Controls — date shortcuts; full column sort lives in table headers */}
      <div className="flex flex-wrap items-center gap-1.5 min-w-0">
        <span className="text-sm text-muted-foreground whitespace-nowrap">
          {t('documents.filter.sortBy')}
        </span>
        <Button
          variant={sortField === 'created_at' ? 'secondary' : 'ghost'}
          size="sm"
          onClick={() => toggleSort('created_at')}
          className="gap-1 h-9"
          data-testid="toolbar-sort-created_at"
        >
          {t('documents.filter.created')}
          {sortField === 'created_at' && (
            sortDirection === 'asc' ? (
              <ArrowUp className="h-3 w-3" />
            ) : (
              <ArrowDown className="h-3 w-3" />
            )
          )}
        </Button>
        <Button
          variant={sortField === 'updated_at' ? 'secondary' : 'ghost'}
          size="sm"
          onClick={() => toggleSort('updated_at')}
          className="gap-1 h-9"
          data-testid="toolbar-sort-updated_at"
        >
          {t('documents.filter.updated')}
          {sortField === 'updated_at' && (
            sortDirection === 'asc' ? (
              <ArrowUp className="h-3 w-3" />
            ) : (
              <ArrowDown className="h-3 w-3" />
            )
          )}
        </Button>
      </div>
    </div>
  );
}
