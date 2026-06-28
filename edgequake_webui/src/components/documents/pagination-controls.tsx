'use client';

import { Button } from '@/components/ui/button';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface PaginationControlsProps {
  currentPage: number;
  totalPages: number;
  pageSize: number;
  totalItems?: number;
  onPageChange: (page: number) => void;
  onPageSizeChange: (size: number) => void;
  pageSizeOptions?: number[];
}

export function PaginationControls({
  currentPage,
  totalPages,
  pageSize,
  totalItems,
  onPageChange,
  onPageSizeChange,
  pageSizeOptions = [10, 20, 50, 100],
}: PaginationControlsProps) {
  const { t } = useTranslation();

  const handleFirstPage = () => onPageChange(1);
  const handlePrevPage = () => onPageChange(Math.max(1, currentPage - 1));
  const handleNextPage = () => onPageChange(Math.min(totalPages, currentPage + 1));
  const handleLastPage = () => onPageChange(totalPages);

  return (
    <div className="flex items-center justify-between px-2 py-3 gap-4">
      {/* Left: showing range — communicates scope at a glance (DI-05) */}
      <div className="flex items-center gap-3 text-sm text-muted-foreground min-w-0">
        {totalItems !== undefined && totalItems > 0 ? (
          <span className="tabular-nums whitespace-nowrap">
            {((currentPage - 1) * pageSize + 1).toLocaleString()}–
            {Math.min(currentPage * pageSize, totalItems).toLocaleString()}
            {' of '}
            <span className="font-medium text-foreground">{totalItems.toLocaleString()}</span>
          </span>
        ) : null}
        <div className="flex items-center gap-1.5">
          <span className="hidden sm:inline">{t('documents.pagination.rowsPerPage', 'Rows')}</span>
          <Select
            value={String(pageSize)}
            onValueChange={(v) => onPageSizeChange(Number(v))}
          >
            <SelectTrigger className="w-16 h-7 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {pageSizeOptions.map((size) => (
                <SelectItem key={size} value={String(size)}>
                  {size}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Right: page navigation */}
      <div className="flex items-center gap-1 shrink-0">
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={handleFirstPage}
          disabled={currentPage <= 1}
          aria-label={t('documents.pagination.firstPage', 'First page')}
        >
          <ChevronsLeft className="h-3.5 w-3.5" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={handlePrevPage}
          disabled={currentPage <= 1}
          aria-label={t('documents.pagination.prevPage', 'Previous page')}
        >
          <ChevronLeft className="h-3.5 w-3.5" />
        </Button>
        <span className="text-xs text-muted-foreground tabular-nums px-1 select-none">
          {currentPage} / {totalPages || 1}
        </span>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={handleNextPage}
          disabled={currentPage >= totalPages}
          aria-label={t('documents.pagination.nextPage', 'Next page')}
        >
          <ChevronRight className="h-3.5 w-3.5" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={handleLastPage}
          disabled={currentPage >= totalPages}
          aria-label={t('documents.pagination.lastPage', 'Last page')}
        >
          <ChevronsRight className="h-3.5 w-3.5" />
        </Button>
      </div>
    </div>
  );
}
