'use client';

import { Button } from '@/components/ui/button';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { ArrowDown, ArrowUp } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export type DocStatus = 'all' | 'pending' | 'processing' | 'completed' | 'failed' | 'partial_failure' | 'cancelled';
export type SortField = 'created_at' | 'updated_at' | 'title' | 'status' | 'entity_count';
export type SortDirection = 'asc' | 'desc';

interface DocumentFiltersProps {
  status: DocStatus;
  sortField: SortField;
  sortDirection: SortDirection;
  onStatusChange: (status: DocStatus) => void;
  onSortFieldChange: (field: SortField) => void;
  onSortDirectionChange: (direction: SortDirection) => void;
  statusCounts?: Record<DocStatus, number>;
}

export function DocumentFilters({
  status,
  sortField,
  sortDirection,
  onStatusChange,
  onSortFieldChange,
  onSortDirectionChange,
  statusCounts,
}: DocumentFiltersProps) {
  const { t } = useTranslation();

  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      onSortDirectionChange(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      onSortFieldChange(field);
      onSortDirectionChange('desc');
    }
  };

  const getStatusLabel = (statusKey: DocStatus) => {
    const label = t(`documents.status.${statusKey}`);
    if (statusCounts && statusCounts[statusKey] !== undefined) {
      return `${label} (${statusCounts[statusKey]})`;
    }
    return label;
  };

  return (
    <div className="flex flex-wrap items-center gap-3">
      {/* Status Filter */}
      <Select
        value={status}
        onValueChange={(v) => onStatusChange(v as DocStatus)}
      >
        <SelectTrigger className="w-40 h-10">
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

      {/* Divider */}
      <div className="h-6 w-px bg-border hidden sm:block" />

      {/* Sort Controls */}
      <div className="flex items-center gap-1.5">
        <span className="text-sm text-muted-foreground whitespace-nowrap">
          {t('documents.filter.sortBy')}
        </span>
        <Button
          variant={sortField === 'created_at' ? 'secondary' : 'ghost'}
          size="sm"
          onClick={() => toggleSort('created_at')}
          className="gap-1 h-9"
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
