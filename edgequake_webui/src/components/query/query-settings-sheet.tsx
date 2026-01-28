/**
 * @module QuerySettingsSheet
 * @description Settings panel for query configuration (extracted from query-interface.tsx for better compilation performance).
 * 
 * Provides UI controls for:
 * - Streaming toggle
 * - Top K results
 * - Temperature
 * - Max tokens
 * 
 * @implements FEAT0007 - Natural Language Query Processing
 * @implements BR0105 - Streaming must show progressive thinking indicators
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import {
    Sheet,
    SheetContent,
    SheetDescription,
    SheetHeader,
    SheetTitle,
    SheetTrigger,
} from '@/components/ui/sheet';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import {
    BookOpen,
    Brain,
    Gauge,
    Info,
    Settings2,
    Sliders,
    Thermometer,
    Zap,
} from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

interface QuerySettings {
  stream: boolean;
  topK: number;
  temperature: number;
  maxTokens: number;
}

interface QuerySettingsSheetProps {
  /** Current query settings */
  settings: QuerySettings;
  /** Callback to update query settings */
  onSettingsChange: (updates: Partial<QuerySettings>) => void;
  /** Whether the settings panel is disabled */
  disabled?: boolean;
  /** Optional trigger button */
  trigger?: ReactNode;
}

export function QuerySettingsSheet({
  settings,
  onSettingsChange,
  disabled = false,
  trigger,
}: QuerySettingsSheetProps) {
  const { t } = useTranslation();

  return (
    <Sheet>
      <SheetTrigger asChild>
        {trigger || (
          <Button variant="ghost" size="icon" disabled={disabled}>
            <Settings2 className="h-4 w-4" />
          </Button>
        )}
      </SheetTrigger>
      <SheetContent className="w-[400px] sm:w-[480px] flex flex-col p-0">
        <SheetHeader className="px-6 py-4 border-b shrink-0">
          <SheetTitle className="flex items-center gap-2 text-base">
            <Sliders className="h-4 w-4 text-primary" />
            {t('query.settings.title', 'Query Settings')}
          </SheetTitle>
          <SheetDescription className="text-xs">
            {t('query.settings.description', 'Configure how the AI processes and responds to your queries.')}
          </SheetDescription>
        </SheetHeader>
        
        <ScrollArea className="flex-1">
          <div className="px-6 py-4 space-y-5">
            {/* Response Mode Section */}
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Zap className="h-3.5 w-3.5 text-amber-500" />
                <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  {t('query.settings.responseMode', 'Response Mode')}
                </h3>
              </div>
              
              <div className="rounded-lg border p-3 space-y-3 bg-muted/20">
                {/* Stream Toggle */}
                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label htmlFor="stream-toggle" className="text-sm font-medium cursor-pointer">
                      {t('query.settings.streaming', 'Streaming')}
                    </Label>
                    <p className="text-[11px] text-muted-foreground leading-tight">
                      {t('query.settings.streamingDescription', 'Show response as it generates')}
                    </p>
                  </div>
                  <Switch
                    id="stream-toggle"
                    checked={settings.stream}
                    onCheckedChange={(stream) => onSettingsChange({ stream })}
                  />
                </div>
              </div>
            </div>

            <Separator />

            {/* Retrieval Section */}
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <BookOpen className="h-3.5 w-3.5 text-blue-500" />
                <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  {t('query.settings.retrieval', 'Retrieval')}
                </h3>
              </div>
              
              <div className="rounded-lg border p-3 space-y-3 bg-muted/20">
                {/* Top K */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-1.5">
                      <Label className="text-sm font-medium">
                        {t('query.settings.topK', 'Top K Results')}
                      </Label>
                      <TooltipProvider>
                        <Tooltip>
                          <TooltipTrigger>
                            <Info className="h-3 w-3 text-muted-foreground" />
                          </TooltipTrigger>
                          <TooltipContent side="top" className="max-w-[200px]">
                            <p className="text-xs">
                              {t('query.settings.topKHint', 'Number of relevant chunks to retrieve from the knowledge graph')}
                            </p>
                          </TooltipContent>
                        </Tooltip>
                      </TooltipProvider>
                    </div>
                    <Badge variant="secondary" className="font-mono text-[10px] h-5 px-1.5">
                      {settings.topK}
                    </Badge>
                  </div>
                  <Slider
                    value={[settings.topK]}
                    onValueChange={([topK]) => onSettingsChange({ topK })}
                    min={1}
                    max={50}
                    step={1}
                    className="w-full"
                  />
                  <div className="flex justify-between text-[10px] text-muted-foreground">
                    <span>1 (Precise)</span>
                    <span>50 (Comprehensive)</span>
                  </div>
                </div>
              </div>
            </div>

            <Separator />

            {/* Generation Section */}
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Brain className="h-3.5 w-3.5 text-purple-500" />
                <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  {t('query.settings.generation', 'Generation')}
                </h3>
              </div>
              
              <div className="rounded-lg border p-3 space-y-4 bg-muted/20">
                {/* Temperature */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-1.5">
                      <Thermometer className="h-3 w-3 text-muted-foreground" />
                      <Label className="text-sm font-medium">
                        {t('query.settings.temperature', 'Temperature')}
                      </Label>
                      <TooltipProvider>
                        <Tooltip>
                          <TooltipTrigger>
                            <Info className="h-3 w-3 text-muted-foreground" />
                          </TooltipTrigger>
                          <TooltipContent side="top" className="max-w-[200px]">
                            <p className="text-xs">
                              {t('query.settings.temperatureHint', 'Controls randomness. Lower = more focused, higher = more creative')}
                            </p>
                          </TooltipContent>
                        </Tooltip>
                      </TooltipProvider>
                    </div>
                    <Badge variant="secondary" className="font-mono text-[10px] h-5 px-1.5">
                      {settings.temperature.toFixed(1)}
                    </Badge>
                  </div>
                  <Slider
                    value={[settings.temperature]}
                    onValueChange={([temperature]) => onSettingsChange({ temperature })}
                    min={0}
                    max={2}
                    step={0.1}
                    className="w-full"
                  />
                  <div className="flex justify-between text-[10px] text-muted-foreground">
                    <span>0 (Precise)</span>
                    <span>2 (Creative)</span>
                  </div>
                </div>

                {/* Max Tokens */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-1.5">
                      <Gauge className="h-3 w-3 text-muted-foreground" />
                      <Label className="text-sm font-medium">
                        {t('query.settings.maxTokens', 'Max Tokens')}
                      </Label>
                      <TooltipProvider>
                        <Tooltip>
                          <TooltipTrigger>
                            <Info className="h-3 w-3 text-muted-foreground" />
                          </TooltipTrigger>
                          <TooltipContent side="top" className="max-w-[200px]">
                            <p className="text-xs">
                              {t('query.settings.maxTokensHint', 'Maximum length of the generated response')}
                            </p>
                          </TooltipContent>
                        </Tooltip>
                      </TooltipProvider>
                    </div>
                    <Badge variant="secondary" className="font-mono text-[10px] h-5 px-1.5">
                      {settings.maxTokens}
                    </Badge>
                  </div>
                  <Slider
                    value={[settings.maxTokens]}
                    onValueChange={([maxTokens]) => onSettingsChange({ maxTokens })}
                    min={256}
                    max={4096}
                    step={256}
                    className="w-full"
                  />
                  <div className="flex justify-between text-[10px] text-muted-foreground">
                    <span>256</span>
                    <span>4096</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </ScrollArea>
      </SheetContent>
    </Sheet>
  );
}
