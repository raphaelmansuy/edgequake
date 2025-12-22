'use client';

import { Target, Globe, Layers, Zap } from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import type { QueryMode } from '@/types';

interface QueryModeSelectorProps {
  value: QueryMode;
  onChange: (mode: QueryMode) => void;
  disabled?: boolean;
}

const modes: {
  id: QueryMode;
  name: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
}[] = [
  {
    id: 'local',
    name: 'Local',
    description: 'Search within specific entity neighborhoods. Best for targeted questions about known topics.',
    icon: Target,
    color: 'text-blue-500',
  },
  {
    id: 'global',
    name: 'Global',
    description: 'Search the entire knowledge graph. Best for broad questions requiring comprehensive context.',
    icon: Globe,
    color: 'text-green-500',
  },
  {
    id: 'hybrid',
    name: 'Hybrid',
    description: 'Combines local and global search for balanced results. Recommended for most queries.',
    icon: Layers,
    color: 'text-purple-500',
  },
  {
    id: 'naive',
    name: 'Simple',
    description: 'Direct LLM query without graph context. Fastest but less accurate.',
    icon: Zap,
    color: 'text-orange-500',
  },
];

export function QueryModeSelector({ value, onChange, disabled }: QueryModeSelectorProps) {
  return (
    <TooltipProvider>
      <div className="flex items-center gap-1 p-1 bg-muted rounded-lg">
        {modes.map((mode) => {
          const Icon = mode.icon;
          const isSelected = value === mode.id;
          
          return (
            <Tooltip key={mode.id}>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  onClick={() => onChange(mode.id)}
                  disabled={disabled}
                  className={cn(
                    'flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition-all',
                    isSelected
                      ? 'bg-background shadow-sm'
                      : 'hover:bg-background/50',
                    disabled && 'opacity-50 cursor-not-allowed'
                  )}
                  aria-label={`Select ${mode.name} query mode`}
                  aria-pressed={isSelected}
                >
                  <Icon className={cn('h-4 w-4', isSelected ? mode.color : 'text-muted-foreground')} />
                  <span className={isSelected ? '' : 'text-muted-foreground'}>{mode.name}</span>
                </button>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="max-w-xs">
                <p className="font-medium">{mode.name} Mode</p>
                <p className="text-xs text-muted-foreground mt-1">{mode.description}</p>
              </TooltipContent>
            </Tooltip>
          );
        })}
      </div>
    </TooltipProvider>
  );
}
