/**
 * @module DocumentSearchBar
 * @description Search input with clear button for document filtering.
 * Extracted from DocumentManager for SRP compliance (OODA-24).
 * 
 * WHY: Search UI was inline in DocumentManager causing bloat.
 * 
 * @implements FEAT0401 - Document search functionality
 */
'use client';

import { Input } from '@/components/ui/input';
import { Search, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

/**
 * Props for DocumentSearchBar component.
 */
export interface DocumentSearchBarProps {
  /** Current search query */
  value: string;
  /** Handler for search query changes */
  onChange: (value: string) => void;
  /** Optional placeholder override */
  placeholder?: string;
}

/**
 * Search input for filtering documents.
 */
export function DocumentSearchBar({ value, onChange, placeholder }: DocumentSearchBarProps) {
  const { t } = useTranslation();

  return (
    <div className="relative flex-1 max-w-md">
      <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
      <Input
        placeholder={placeholder || t('documents.search.placeholder', 'Search documents...')}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="pl-9 pr-8 h-9"
        aria-label="Search documents"
      />
      {/* OODA-36: Clear search button */}
      {value && (
        <button
          type="button"
          onClick={() => onChange('')}
          className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded hover:bg-muted transition-colors"
          aria-label="Clear search"
        >
          <X className="h-3.5 w-3.5 text-muted-foreground" />
        </button>
      )}
    </div>
  );
}

export default DocumentSearchBar;
