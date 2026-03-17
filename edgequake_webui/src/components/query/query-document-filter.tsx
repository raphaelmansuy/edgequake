/**
 * @module QueryDocumentFilter
 * @description Popover filter panel for restricting RAG queries to specific documents.
 * Supports date range (from/to) and document name pattern filters.
 *
 * @implements SPEC-005: Document filters for queries
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import type { DocumentFilter } from '@/types';
import { Filter, X } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface QueryDocumentFilterProps {
  /** Current filter value */
  value?: DocumentFilter;
  /** Callback when filter changes */
  onChange: (filter: DocumentFilter | undefined) => void;
  /** Whether the filter is disabled */
  disabled?: boolean;
}

/** Count how many active filter criteria are set. */
function activeFilterCount(filter?: DocumentFilter): number {
  if (!filter) return 0;
  let count = 0;
  if (filter.date_from) count++;
  if (filter.date_to) count++;
  if (filter.document_pattern) count++;
  return count;
}

export function QueryDocumentFilter({
  value,
  onChange,
  disabled = false,
}: QueryDocumentFilterProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  const count = activeFilterCount(value);

  const handleFieldChange = useCallback(
    (field: keyof DocumentFilter, val: string) => {
      const next: DocumentFilter = { ...value };
      if (val) {
        next[field] = val;
      } else {
        delete next[field];
      }
      // If all fields are empty, clear the filter entirely
      if (!next.date_from && !next.date_to && !next.document_pattern) {
        onChange(undefined);
      } else {
        onChange(next);
      }
    },
    [value, onChange],
  );

  const handleClear = useCallback(() => {
    onChange(undefined);
  }, [onChange]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant={count > 0 ? 'secondary' : 'ghost'}
          size="icon"
          disabled={disabled}
          aria-label={t('query.filter.toggle', 'Document Filter')}
          className="relative"
        >
          <Filter className="h-4 w-4" />
          {count > 0 && (
            <Badge
              variant="destructive"
              className="absolute -top-1 -right-1 h-4 w-4 p-0 flex items-center justify-center text-[10px]"
            >
              {count}
            </Badge>
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80 space-y-4">
        <div className="flex items-center justify-between">
          <h4 className="text-sm font-semibold">
            {t('query.filter.title', 'Document Filter')}
          </h4>
          {count > 0 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClear}
              className="h-6 text-xs gap-1"
            >
              <X className="h-3 w-3" />
              {t('query.filter.clear', 'Clear')}
            </Button>
          )}
        </div>

        <p className="text-xs text-muted-foreground">
          {t('query.filter.description', 'Restrict RAG context to documents matching these criteria.')}
        </p>

        {/* Date From */}
        <div className="space-y-1.5">
          <Label htmlFor="filter-date-from" className="text-xs">
            {t('query.filter.dateFrom', 'From Date')}
          </Label>
          <Input
            id="filter-date-from"
            type="date"
            value={value?.date_from ?? ''}
            onChange={(e) => handleFieldChange('date_from', e.target.value)}
            className="h-8 text-sm"
          />
        </div>

        {/* Date To */}
        <div className="space-y-1.5">
          <Label htmlFor="filter-date-to" className="text-xs">
            {t('query.filter.dateTo', 'To Date')}
          </Label>
          <Input
            id="filter-date-to"
            type="date"
            value={value?.date_to ?? ''}
            onChange={(e) => handleFieldChange('date_to', e.target.value)}
            className="h-8 text-sm"
          />
        </div>

        {/* Document Pattern */}
        <div className="space-y-1.5">
          <Label htmlFor="filter-pattern" className="text-xs">
            {t('query.filter.pattern', 'Document Name Pattern')}
          </Label>
          <Input
            id="filter-pattern"
            type="text"
            placeholder={t('query.filter.patternPlaceholder', 'e.g. report, invoice')}
            value={value?.document_pattern ?? ''}
            onChange={(e) => handleFieldChange('document_pattern', e.target.value)}
            className="h-8 text-sm"
          />
          <p className="text-[10px] text-muted-foreground">
            {t('query.filter.patternHint', 'Comma-separated terms. Matches titles case-insensitively.')}
          </p>
        </div>
      </PopoverContent>
    </Popover>
  );
}
