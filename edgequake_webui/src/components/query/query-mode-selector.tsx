/**
 * @module QueryModeSelector
 * @description Query mode toggle for selecting RAG retrieval strategy.
 * Surfaces all backend modes (local, global, hybrid, mix, naive, bypass)
 * with explanatory tooltips.
 *
 * @implements FEAT0101 - Naive mode (chunk RAG)
 * @implements FEAT0102 - Local mode (entity neighborhood)
 * @implements FEAT0103 - Global mode (relationship / theme)
 * @implements FEAT0104 - Hybrid mode (local + global [+ naive in EQ])
 * @implements FEAT0105 - Mix mode (full blend, recommended)
 * @implements FEAT0106 - Bypass mode (LLM only)
 *
 * @enforces BR0101 - Mode selection persists across sessions
 * @enforces BR0102 - Mode change updates query behavior immediately
 */
'use client';

import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { QUERY_MODE_META } from '@/lib/query/query-mode-meta';
import { cn } from '@/lib/utils';
import type { QueryMode } from '@/types';
import { useTranslation } from 'react-i18next';

interface QueryModeSelectorProps {
  value: QueryMode;
  onChange: (mode: QueryMode) => void;
  disabled?: boolean;
}

export function QueryModeSelector({ value, onChange, disabled }: QueryModeSelectorProps) {
  const { t } = useTranslation();

  return (
    <TooltipProvider delayDuration={200}>
      <div
        className="flex flex-wrap items-center gap-1 p-1 bg-muted rounded-lg min-w-0 max-w-full"
        role="group"
        aria-label={t('query.modes.groupLabel', 'Query retrieval mode')}
        data-tour="query-mode"
        data-testid="query-mode-selector"
      >
        {QUERY_MODE_META.map((mode) => {
          const Icon = mode.icon;
          const isSelected = value === mode.id;
          const label = t(`query.modes.${mode.id}`, mode.label);
          const description = t(
            `query.modes.${mode.id}Description`,
            mode.description,
          );
          const recommended = Boolean(mode.recommended);

          return (
            <Tooltip key={mode.id}>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  onClick={() => onChange(mode.id)}
                  disabled={disabled}
                  data-testid={`query-mode-${mode.id}`}
                  data-mode={mode.id}
                  className={cn(
                    'relative flex items-center gap-1.5 px-2.5 py-1.5 rounded-md text-sm font-medium transition-all',
                    isSelected
                      ? 'bg-background shadow-sm'
                      : 'hover:bg-background/50',
                    disabled && 'opacity-50 cursor-not-allowed',
                  )}
                  aria-label={
                    recommended
                      ? t(
                          'query.modes.selectRecommended',
                          'Select {{label}} query mode (Recommended)',
                          { label },
                        )
                      : t('query.modes.select', 'Select {{label}} query mode', {
                          label,
                        })
                  }
                  aria-pressed={isSelected}
                >
                  <Icon
                    className={cn(
                      'h-4 w-4 shrink-0',
                      isSelected ? mode.color : 'text-muted-foreground',
                    )}
                  />
                  <span
                    className={cn(
                      'whitespace-nowrap',
                      isSelected ? '' : 'text-muted-foreground',
                    )}
                  >
                    {label}
                  </span>
                  {recommended && !isSelected && (
                    <span
                      className="absolute top-1 right-1 h-1.5 w-1.5 rounded-full bg-primary/60"
                      aria-hidden="true"
                    />
                  )}
                </button>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="max-w-sm space-y-1.5 p-3">
                <p className="font-semibold text-foreground">
                  {label}
                  {recommended
                    ? t('query.modes.recommendedSuffix', ' · Recommended')
                    : ''}
                </p>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  {description}
                </p>
                <p className="text-[11px] uppercase tracking-wide text-foreground/90 font-medium">
                  {t('query.modes.apiName', 'API mode')}: {mode.apiName}
                </p>
              </TooltipContent>
            </Tooltip>
          );
        })}
      </div>
    </TooltipProvider>
  );
}
