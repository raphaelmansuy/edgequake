/**
 * @module ChatMessage
 * @description Chat message component with streaming support and source citations.
 * Renders user/assistant messages with chain-of-thought, metrics, and copy actions.
 * 
 * @implements UC0203 - Display response with source citations
 * @implements FEAT0734 - Chain-of-thought thinking display
 * @implements FEAT0302 - Message regeneration capability
 * @implements FEAT0303 - Token usage and duration metrics
 * 
 * @enforces BR0104 - All responses include clickable source citations
 * @enforces BR0105 - Streaming progress shows thinking indicators
 * 
 * @see {@link docs/features.md} FEAT0734
 */
'use client';

import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import type { QueryContext } from '@/types';
import {
    Brain,
    Check,
    ChevronDown,
    ChevronRight,
    Clock,
    Copy,
    Gauge,
    RefreshCw,
    Sparkles,
    User,
    Zap
} from 'lucide-react';
import { memo, useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { StreamingMarkdownRenderer } from './markdown';
import { SourceCitations } from './source-citations';
import { parseCOTContent } from './thinking-display';

export interface ChatMessageData {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp?: number;
  isStreaming?: boolean;
  isError?: boolean;
  mode?: 'local' | 'global' | 'hybrid' | 'naive';
  tokensUsed?: number;
  durationMs?: number;
  thinkingTimeMs?: number;
  context?: QueryContext;
  /** LLM provider used (lineage tracking). @implements SPEC-032 */
  llmProvider?: string;
  /** LLM model used (lineage tracking). @implements SPEC-032 */
  llmModel?: string;
}

interface ChatMessageProps {
  message: ChatMessageData;
  isLast?: boolean;
  onCopy?: () => void;
  onRegenerate?: () => void;
  showMetadata?: boolean;
}

/**
 * User Message Bubble
 */
const UserMessage = memo(function UserMessage({
  message,
}: {
  message: ChatMessageData;
}) {
  return (
    <div className="flex justify-end mb-6 animate-slide-in-right">
      <div className="flex items-start gap-3 max-w-[85%]">
        <div 
          className={cn(
            'rounded-2xl rounded-tr-sm px-4 py-3',
            'bg-gradient-to-br from-primary to-primary/90',
            'text-primary-foreground',
            'shadow-[0_2px_8px_rgba(0,0,0,0.08)]',
            'dark:shadow-[0_2px_8px_rgba(0,0,0,0.2)]'
          )}
        >
          <p className="whitespace-pre-wrap break-words overflow-wrap-anywhere leading-relaxed">
            {message.content}
          </p>
        </div>
        <Avatar className="h-8 w-8 shrink-0 ring-2 ring-background shadow-sm">
          <AvatarFallback className="bg-primary/10">
            <User className="h-4 w-4" />
          </AvatarFallback>
        </Avatar>
      </div>
    </div>
  );
});

/**
 * Thinking/Reasoning Section
 */
const ThinkingSection = memo(function ThinkingSection({
  thinking,
  thinkingTimeMs,
  isExpanded,
  onToggle,
}: {
  thinking: string[];
  thinkingTimeMs?: number;
  isExpanded: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation();

  if (thinking.length === 0) return null;

  return (
    <div 
      className={cn(
        'rounded-xl border overflow-hidden',
        'bg-[oklch(0.97_0.01_280)] dark:bg-[oklch(0.18_0.01_280)]',
        'border-[oklch(0.9_0.02_280)] dark:border-[oklch(0.3_0.02_280)]'
      )}
    >
      <button
        onClick={onToggle}
        className={cn(
          'flex items-center gap-2 w-full px-4 py-3 text-left',
          'hover:bg-muted/30 transition-colors'
        )}
        aria-expanded={isExpanded}
      >
        {isExpanded ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        )}
        <div className="relative">
          <Brain className="h-4 w-4 text-primary/70" />
          <span className="absolute -top-0.5 -right-0.5 flex h-1.5 w-1.5">
            <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-primary animate-pulse" />
          </span>
        </div>
        <span className="text-sm font-medium text-foreground/80">
          {t('query.reasoning', 'Reasoning')}
        </span>
        {thinkingTimeMs && (
          <span className="text-xs text-muted-foreground ml-auto flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {(thinkingTimeMs / 1000).toFixed(1)}s
          </span>
        )}
      </button>
      
      {isExpanded && (
        <div 
          className={cn(
            'px-4 pb-4 pt-0',
            'border-t border-[oklch(0.9_0.02_280)] dark:border-[oklch(0.3_0.02_280)]'
          )}
        >
          <div 
            className={cn(
              'text-sm text-muted-foreground whitespace-pre-wrap',
              'pl-4 pt-3',
              'border-l-2 border-primary/30'
            )}
          >
            {thinking.join('\n\n')}
          </div>
        </div>
      )}
    </div>
  );
});

/**
 * Message Metadata Bar
 */
const MetadataBar = memo(function MetadataBar({
  mode,
  tokensUsed,
  durationMs,
  llmProvider,
  llmModel,
  copied,
  onCopy,
  onRegenerate,
  isLast,
  isVisible,
}: {
  mode?: string;
  tokensUsed?: number;
  durationMs?: number;
  llmProvider?: string;
  llmModel?: string;
  copied: boolean;
  onCopy: () => void;
  onRegenerate?: () => void;
  isLast?: boolean;
  isVisible: boolean;
}) {
  const { t } = useTranslation();

  return (
    <div 
      className={cn(
        'flex items-center gap-2 pt-2 transition-opacity duration-200',
        isVisible ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
      )}
    >
      {/* Stats */}
      <div className="flex items-center gap-2.5 text-xs text-muted-foreground flex-wrap">
        {mode && (
          <Badge 
            variant="outline" 
            className={cn(
              'text-xs font-normal px-2 py-0.5',
              'bg-muted/50'
            )}
          >
            {mode}
          </Badge>
        )}
        {/* SPEC-032: Display LLM provider/model as lineage badge */}
        {(llmProvider || llmModel) && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Badge 
                  variant="secondary" 
                  className={cn(
                    'text-xs font-normal px-2 py-0.5',
                    'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300',
                    'border-blue-200 dark:border-blue-800'
                  )}
                >
                  <Brain className="h-3 w-3 mr-1" />
                  {llmProvider || 'default'}
                  {llmModel && `: ${llmModel.split(':')[0]}`}
                </Badge>
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">
                  {t('query.llmLineage', 'LLM Provider')}: {llmProvider || 'server default'}
                  {llmModel && <><br />{t('query.llmModel', 'Model')}: {llmModel}</>}
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
        {tokensUsed && (
          <span className="flex items-center gap-1" title={t('query.tokensUsed', 'Tokens used')}>
            <Zap className="h-3 w-3" />
            {tokensUsed.toLocaleString()}
          </span>
        )}
        {durationMs && (
          <span className="flex items-center gap-1" title={t('query.duration', 'Generation time')}>
            <Clock className="h-3 w-3" />
            {(durationMs / 1000).toFixed(1)}s
          </span>
        )}
        {/* SPEC-032: Show tokens per second with model name for performance insight */}
        {tokensUsed && durationMs && durationMs > 0 && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="flex items-center gap-1 text-emerald-600 dark:text-emerald-400" title={t('query.tokensPerSecond', 'Tokens per second')}>
                  <Gauge className="h-3 w-3" />
                  {((tokensUsed / durationMs) * 1000).toFixed(1)}/s
                  {/* REQ-22: Display model after tokens/second */}
                  {(llmProvider || llmModel) && (
                    <span className="text-muted-foreground">
                      • {llmProvider && llmModel ? `${llmProvider}/${llmModel}` : llmProvider || llmModel}
                    </span>
                  )}
                </span>
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">
                  {t('query.tokensPerSecondDesc', 'Generation speed')}: {((tokensUsed / durationMs) * 1000).toFixed(1)} {t('query.tokensPerSecondUnit', 'tokens/second')}
                  {(llmProvider || llmModel) && (
                    <>
                      <br />
                      {t('query.modelUsed', 'Model')}: {llmProvider && llmModel ? `${llmProvider}/${llmModel}` : llmProvider || llmModel}
                    </>
                  )}
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 ml-auto">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className={cn(
                  'h-7 w-7',
                  copied && 'text-green-500'
                )}
                onClick={onCopy}
              >
                {copied ? (
                  <Check className="h-3.5 w-3.5" />
                ) : (
                  <Copy className="h-3.5 w-3.5" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              {copied ? t('common.copied', 'Copied!') : t('common.copy', 'Copy')}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>

        {isLast && onRegenerate && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7"
                  onClick={onRegenerate}
                >
                  <RefreshCw className="h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                {t('query.regenerate', 'Regenerate')}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </div>
    </div>
  );
});

/**
 * Streaming Indicator - Minimal, smooth animation
 */
const StreamingIndicator = memo(function StreamingIndicator() {
  const { t } = useTranslation();
  
  return (
    <div 
      className={cn(
        'rounded-2xl rounded-tl-sm px-4 py-3',
        'bg-card border border-border',
        'shadow-sm'
      )}
    >
      <div className="flex items-center gap-2 text-muted-foreground">
        {/* Simple pulsing dot - no expanding ring */}
        <span className="inline-flex h-2 w-2 rounded-full bg-primary animate-pulse" />
        <span className="text-sm">
          {t('query.generating', 'Generating response...')}
        </span>
      </div>
    </div>
  );
});

/**
 * Assistant Message Bubble
 */
const AssistantMessage = memo(function AssistantMessage({
  message,
  isLast,
  onCopy,
  onRegenerate,
  showMetadata = true,
}: ChatMessageProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [thinkingExpanded, setThinkingExpanded] = useState(false);

  const handleCopy = useCallback(async () => {
    const parsed = parseCOTContent(message.content);
    const textToCopy = parsed.response || message.content;
    try {
      await navigator.clipboard.writeText(textToCopy);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
      onCopy?.();
    } catch (err) {
      console.error('Copy failed:', err);
    }
  }, [message.content, onCopy]);

  const toggleThinking = useCallback(() => {
    setThinkingExpanded(prev => !prev);
  }, []);

  // Parse Chain-of-Thought content
  const parsed = parseCOTContent(message.content);
  const hasThinking = parsed.thinking.length > 0;
  const displayContent = parsed.response;

  return (
    <div className="flex justify-start mb-6 group animate-slide-in-left">
      <div className="flex items-start gap-3 max-w-[85%] min-w-0">
        {/* Avatar */}
        <Avatar 
          className={cn(
            'h-9 w-9 shrink-0 mt-1',
            'ring-2 ring-primary/20 shadow-sm'
          )}
        >
          <AvatarFallback 
            className={cn(
              'bg-gradient-to-br from-primary/80 to-primary',
              'text-primary-foreground'
            )}
          >
            <Sparkles className="h-4 w-4" />
          </AvatarFallback>
        </Avatar>

        <div className="space-y-3 min-w-0 flex-1">
          {/* Header with model name */}
          <div className="flex items-center gap-2 text-sm">
            <span className="font-medium text-foreground">EdgeQuake</span>
            {message.timestamp && (
              <span className="text-xs text-muted-foreground">
                {new Date(message.timestamp).toLocaleTimeString([], { 
                  hour: '2-digit', 
                  minute: '2-digit' 
                })}
              </span>
            )}
          </div>

          {/* Thinking Section */}
          {hasThinking && (
            <ThinkingSection
              thinking={parsed.thinking}
              thinkingTimeMs={message.thinkingTimeMs}
              isExpanded={thinkingExpanded}
              onToggle={toggleThinking}
            />
          )}

          {/* Main Response Content */}
          {(displayContent || message.isStreaming) && (
            <div 
              className={cn(
                'rounded-2xl rounded-tl-sm px-4 py-3',
                'bg-card border border-border/60',
                'shadow-[0_1px_4px_rgba(0,0,0,0.04)]',
                'dark:shadow-[0_1px_4px_rgba(0,0,0,0.1)]'
              )}
            >
              {message.isError ? (
                <p className="text-destructive break-words overflow-wrap-anywhere">
                  {displayContent}
                </p>
              ) : displayContent ? (
                <div className="break-words overflow-wrap-anywhere hyphens-auto">
                  <StreamingMarkdownRenderer
                    content={displayContent}
                    isStreaming={message.isStreaming}
                    className="prose-sm"
                  />
                </div>
              ) : null}
              
              {/* Streaming cursor removed - was causing visual artifacts */}
            </div>
          )}

          {/* Streaming indicator when in thinking phase */}
          {message.isStreaming && !displayContent && hasThinking && (
            <StreamingIndicator />
          )}

          {/* Metadata & Actions */}
          {showMetadata && !message.isStreaming && displayContent && (
            <MetadataBar
              mode={message.mode}
              tokensUsed={message.tokensUsed}
              durationMs={message.durationMs}
              llmProvider={message.llmProvider}
              llmModel={message.llmModel}
              copied={copied}
              onCopy={handleCopy}
              onRegenerate={onRegenerate}
              isLast={isLast}
              isVisible={!!isLast}
            />
          )}

          {/* Source Citations */}
          {message.context && !message.isStreaming && (
            <div className="mt-2">
              <SourceCitations
                context={message.context}
                onEntityClick={(entityId) => {
                  window.location.href = `/graph?entity=${encodeURIComponent(entityId)}`;
                }}
                onDocumentClick={(documentId, chunkContent, chunkIndex, startLine, endLine) => {
                  // Navigate to document detail page with line numbers and/or highlight
                  const url = new URL(`/documents/${encodeURIComponent(documentId)}`, window.location.origin);
                  
                  // Priority 1: Use line numbers if available
                  if (startLine !== undefined && endLine !== undefined) {
                    url.searchParams.set('start_line', startLine.toString());
                    url.searchParams.set('end_line', endLine.toString());
                  }
                  
                  // Fallback: Use text highlight if no line numbers
                  if (chunkContent && startLine === undefined) {
                    // Use first 100 chars of chunk content as highlight search term
                    const searchTerm = chunkContent.slice(0, 100);
                    url.searchParams.set('highlight', searchTerm);
                  }
                  
                  window.location.href = url.toString();
                }}
                onExploreGraph={(entityLabels) => {
                  // Navigate to graph page with entity filter
                  const url = new URL('/graph', window.location.origin);
                  if (entityLabels.length > 0) {
                    // Pass entities as comma-separated list for filtering
                    url.searchParams.set('entities', entityLabels.join(','));
                    // Use first entity as focus node
                    url.searchParams.set('focus', entityLabels[0]);
                  }
                  window.location.href = url.toString();
                }}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
});

/**
 * ChatMessage Component - Unified message display
 */
export const ChatMessage = memo(function ChatMessage({
  message,
  isLast,
  onCopy,
  onRegenerate,
  showMetadata = true,
}: ChatMessageProps) {
  if (message.role === 'user') {
    return <UserMessage message={message} />;
  }

  return (
    <AssistantMessage
      message={message}
      isLast={isLast}
      onCopy={onCopy}
      onRegenerate={onRegenerate}
      showMetadata={showMetadata}
    />
  );
});

export default ChatMessage;
