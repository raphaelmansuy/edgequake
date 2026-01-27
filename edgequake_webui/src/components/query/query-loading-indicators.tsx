/**
 * @module QueryLoadingIndicators
 * @description Loading indicators for query interface (extracted from query-interface.tsx for better compilation performance).
 * 
 * Provides:
 * - Simple loading message for streaming
 * - Multi-phase loading indicator for non-streaming
 * 
 * @implements FEAT0007 - Natural Language Query Processing
 * @implements BR0105 - Streaming must show progressive thinking indicators
 */
'use client';

import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Brain, Search, Sparkles } from 'lucide-react';
import { memo, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

/**
 * Delightful Loading Indicator - Shows minimal, smooth placeholder while waiting
 */
export const LoadingMessage = memo(function LoadingMessage() {
  const { t } = useTranslation();
  
  return (
    <div className="flex justify-start mb-4 animate-fade-in">
      <div className="flex items-start gap-3 max-w-[85%]">
        <Avatar className="h-8 w-8 shrink-0 mt-1">
          <AvatarFallback className="bg-gradient-to-br from-primary/80 to-primary">
            <Sparkles className="h-4 w-4 text-primary-foreground" />
          </AvatarFallback>
        </Avatar>

        <div className="min-w-0 flex-1">
          <div className="bg-card border rounded-2xl rounded-tl-sm px-4 py-3">
            <div className="flex items-center gap-3">
              {/* Simple status indicator - subtle dot that pulses */}
              <div className="relative flex items-center gap-2">
                <span className="inline-flex h-2 w-2 rounded-full bg-primary animate-pulse" />
                <span className="text-sm text-muted-foreground">
                  {t('query.processing', 'Processing your query...')}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});

/**
 * Non-Streaming Loading Indicator - Delightful multi-phase animation
 * Shows a sophisticated loading experience with visual progression
 */
export const NonStreamingLoadingIndicator = memo(function NonStreamingLoadingIndicator() {
  const { t } = useTranslation();
  const [phase, setPhase] = useState(0);
  
  const phases = [
    { icon: Search, text: t('query.loading.searching', 'Searching knowledge graph...') },
    { icon: Brain, text: t('query.loading.analyzing', 'Analyzing relevant context...') },
    { icon: Sparkles, text: t('query.loading.generating', 'Generating response...') },
  ];

  useEffect(() => {
    const interval = setInterval(() => {
      setPhase((prev) => (prev + 1) % phases.length);
    }, 2000);
    return () => clearInterval(interval);
  }, [phases.length]);

  const CurrentIcon = phases[phase].icon;
  const currentText = phases[phase].text;

  return (
    <div className="flex justify-start mb-4 animate-fade-in">
      <div className="flex items-start gap-3 max-w-[85%]">
        <Avatar className="h-9 w-9 shrink-0 mt-1 ring-2 ring-primary/20 shadow-sm">
          <AvatarFallback className="bg-gradient-to-br from-primary/80 to-primary">
            <Sparkles className="h-4 w-4 text-primary-foreground" />
          </AvatarFallback>
        </Avatar>

        <div className="min-w-0 flex-1 space-y-3">
          {/* Header */}
          <div className="flex items-center gap-2 text-sm">
            <span className="font-medium text-foreground">EdgeQuake</span>
          </div>
          
          {/* Loading Card */}
          <div className="bg-card border border-border/60 rounded-2xl rounded-tl-sm px-4 py-4 shadow-[0_1px_4px_rgba(0,0,0,0.04)] dark:shadow-[0_1px_4px_rgba(0,0,0,0.1)]">
            {/* Phase indicator with smooth transition */}
            <div className="flex items-center gap-3">
              <div className="relative">
                {/* Animated ring around icon */}
                <div className="absolute -inset-1 rounded-full bg-gradient-to-r from-primary/30 to-primary/10 animate-pulse" />
                <div className="relative flex items-center justify-center h-8 w-8 rounded-full bg-primary/10">
                  <CurrentIcon className="h-4 w-4 text-primary animate-pulse" />
                </div>
              </div>
              
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium text-foreground transition-all duration-300">
                  {currentText}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});
