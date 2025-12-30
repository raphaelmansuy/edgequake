'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
    CommandSeparator,
} from '@/components/ui/command';
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '@/components/ui/popover';
import { useDebounce } from '@/hooks/use-debounce';
import { getPopularLabels, searchLabels, type PopularLabel } from '@/lib/api/edgequake';
import { useQuery } from '@tanstack/react-query';
import { Search, Sparkles, X } from 'lucide-react';
import { useCallback, useState } from 'react';

interface LabelSearchProps {
  /** Callback when a label is selected */
  onSelect: (label: string) => void;
  /** Currently selected label (for display) */
  selectedLabel?: string | null;
  /** Callback to clear selection */
  onClear?: () => void;
  /** Placeholder text */
  placeholder?: string;
}

/**
 * LabelSearch - Autocomplete search for graph labels/entities.
 * 
 * Features:
 * - Debounced search (300ms)
 * - Popular labels shown when empty
 * - Shows entity type and degree for context
 * - Server-side filtering for large graphs
 */
export function LabelSearch({
  onSelect,
  selectedLabel,
  onClear,
  placeholder = 'Focus on entity...',
}: LabelSearchProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const debouncedQuery = useDebounce(query, 300);

  // Fetch matching labels from search
  const { data: searchResults, isLoading: isSearching } = useQuery({
    queryKey: ['labels-search', debouncedQuery],
    queryFn: () => searchLabels(debouncedQuery, 20),
    enabled: debouncedQuery.length >= 2,
    staleTime: 30000, // Cache for 30 seconds
  });

  // Fetch popular labels for quick access
  const { data: popularLabels, isLoading: isLoadingPopular } = useQuery({
    queryKey: ['labels-popular'],
    queryFn: () => getPopularLabels({ limit: 10 }),
    staleTime: 60000, // Cache for 1 minute
  });

  const handleSelect = useCallback((label: string) => {
    onSelect(label);
    setOpen(false);
    setQuery('');
  }, [onSelect]);

  const handleClear = useCallback(() => {
    onClear?.();
    setQuery('');
  }, [onClear]);

  // Get entity type color based on type name
  const getTypeColor = (entityType: string): string => {
    const typeColors: Record<string, string> = {
      PERSON: 'bg-blue-500/20 text-blue-600 dark:text-blue-400',
      ORGANIZATION: 'bg-green-500/20 text-green-600 dark:text-green-400',
      LOCATION: 'bg-amber-500/20 text-amber-600 dark:text-amber-400',
      EVENT: 'bg-purple-500/20 text-purple-600 dark:text-purple-400',
      CONCEPT: 'bg-cyan-500/20 text-cyan-600 dark:text-cyan-400',
      TECHNOLOGY: 'bg-rose-500/20 text-rose-600 dark:text-rose-400',
      PRODUCT: 'bg-orange-500/20 text-orange-600 dark:text-orange-400',
    };
    return typeColors[entityType] || 'bg-gray-500/20 text-gray-600 dark:text-gray-400';
  };

  const isLoading = isSearching || isLoadingPopular;
  const showSearch = debouncedQuery.length >= 2;
  const showPopular = !showSearch && popularLabels?.labels && popularLabels.labels.length > 0;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-full max-w-[200px] justify-between h-8 text-xs"
        >
          {selectedLabel ? (
            <span className="truncate">{selectedLabel}</span>
          ) : (
            <span className="text-muted-foreground">{placeholder}</span>
          )}
          <div className="flex items-center gap-1 ml-1">
            {selectedLabel && onClear && (
              <X
                className="h-3 w-3 shrink-0 opacity-50 hover:opacity-100"
                onClick={(e) => {
                  e.stopPropagation();
                  handleClear();
                }}
              />
            )}
            <Search className="h-3 w-3 shrink-0 opacity-50" />
          </div>
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[280px] p-0" align="start">
        <Command shouldFilter={false}>
          <CommandInput
            placeholder="Search entities..."
            value={query}
            onValueChange={setQuery}
            className="h-9"
          />
          <CommandList>
            {/* Loading state */}
            {isLoading && (
              <div className="flex items-center justify-center py-6">
                <div className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
              </div>
            )}

            {/* Empty state */}
            {!isLoading && showSearch && (!searchResults?.labels || searchResults.labels.length === 0) && (
              <CommandEmpty>No entities found.</CommandEmpty>
            )}

            {/* Popular labels (shown when not searching) */}
            {!isLoading && showPopular && (
              <CommandGroup heading="Popular Entities">
                {popularLabels.labels.map((label: PopularLabel) => (
                  <CommandItem
                    key={label.label}
                    value={label.label}
                    onSelect={() => handleSelect(label.label)}
                    className="flex items-center gap-2"
                  >
                    <Sparkles className="h-3 w-3 text-amber-500" />
                    <Badge
                      variant="secondary"
                      className={`text-[10px] px-1.5 py-0 ${getTypeColor(label.entity_type)}`}
                    >
                      {label.entity_type}
                    </Badge>
                    <span className="flex-1 truncate text-sm">{label.label}</span>
                    <span className="text-xs text-muted-foreground tabular-nums">
                      {label.degree}
                    </span>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}

            {/* Search results */}
            {!isLoading && showSearch && searchResults?.labels && searchResults.labels.length > 0 && (
              <CommandGroup heading="Search Results">
                {searchResults.labels.map((label: string) => (
                  <CommandItem
                    key={label}
                    value={label}
                    onSelect={() => handleSelect(label)}
                    className="flex items-center gap-2"
                  >
                    <Search className="h-3 w-3 text-muted-foreground" />
                    <span className="flex-1 truncate text-sm">{label}</span>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}

            {/* Clear selection action */}
            {selectedLabel && (
              <>
                <CommandSeparator />
                <CommandGroup>
                  <CommandItem
                    onSelect={handleClear}
                    className="text-muted-foreground"
                  >
                    <X className="h-3 w-3 mr-2" />
                    Clear selection (show all)
                  </CommandItem>
                </CommandGroup>
              </>
            )}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
