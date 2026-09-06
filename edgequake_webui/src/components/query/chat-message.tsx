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

import { LangfuseOpenSessionLink } from '@/components/settings/langfuse-open-trace-link';
import { useActiveConversationId } from '@/stores/use-query-ui-store';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { getQueryModeMeta } from '@/lib/query/query-mode-meta';
import {
  displayQueryAnswer,
  isZeroAuthzAnswer,
  ZERO_AUTHZ_HELP,
} from '@/lib/query/query-empty-copy';
import { buildDocumentCitationUrl } from '@/lib/utils/document-url';
import { cn } from '@/lib/utils';
import type { QueryContext, QueryMode } from '@/types';
import { isQueryMode } from '@/types/query';
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
import { useRouter } from 'next/navigation';
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
  mode?: QueryMode;
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
    <div
      className="flex justify-end mb-6 motion-safe:animate-slide-in-right"
      role="article"
      aria-label="Your message"
    >
      <div className="flex items-start gap-3 max-w-[95%] sm:max-w-[85%]">
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
            <User className="h-4 w-4" aria-hidden="true" />
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
          'hover:bg-muted/30 transition-colors',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50 focus-visible:ring-offset-1'
        )}
        aria-expanded={isExpanded}
        aria-label={t('query.toggleReasoning', 'Toggle reasoning details')}
      >
        {isExpanded ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        )}
        <div className="relative" aria-hidden="true">
          <Brain className="h-4 w-4 text-primary/70" />
          <span className="absolute -top-0.5 -right-0.5 flex h-1.5 w-1.5">
            <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-primary motion-safe:animate-pulse" />
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
  sessionId,
  copied,
  onCopy,
  onRegenerate,
  isLast,
  isVisible,
}: {
  mode?: QueryMode | string;
  tokensUsed?: number;
  durationMs?: number;
  llmProvider?: string;
  llmModel?: string;
  /** Langfuse session = durable conversation_id (SPEC-124). */
  sessionId?: string | null;
  copied: boolean;
  onCopy: () => void;
  onRegenerate?: () => void;
  isLast?: boolean;
  isVisible: boolean;
}) {
  const { t } = useTranslation();
  const modeMeta =
    mode && isQueryMode(String(mode)) ? getQueryModeMeta(String(mode) as QueryMode) : null;
  const ModeIcon = modeMeta?.icon;
  const modelLabel =
    llmProvider && llmModel
      ? `${llmProvider}/${llmModel}`
      : llmProvider || llmModel || undefined;
  const tokensPerSecond =
    tokensUsed && durationMs && durationMs > 0
      ? ((tokensUsed / durationMs) * 1000).toFixed(1)
      : null;

  return (
    <div 
      className={cn(
        'flex items-center gap-2 pt-2 transition-opacity duration-200',
        isVisible ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
      )}
      data-testid="query-message-metadata"
    >
      {/* Stats: tokens · duration · tks/s · mode · provider/model */}
      <div className="flex items-center gap-2.5 text-xs text-muted-foreground flex-wrap">
        {tokensUsed && (
          <span className="flex items-center gap-1" title={t('query.tokensUsed', 'Tokens used')}>
            <Zap className="h-3 w-3" aria-hidden="true" />
            <span className="sr-only">{t('query.tokensUsed', 'Tokens used')}:</span>
            {tokensUsed.toLocaleString()}
          </span>
        )}
        {durationMs && (
          <span className="flex items-center gap-1" title={t('query.duration', 'Generation time')}>
            <Clock className="h-3 w-3" aria-hidden="true" />
            <span className="sr-only">{t('query.duration', 'Generation time')}:</span>
            {(durationMs / 1000).toFixed(1)}s
          </span>
        )}
        {tokensPerSecond && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <span
                  className="flex items-center gap-1 text-emerald-600 dark:text-emerald-400"
                  title={t('query.tokensPerSecond', 'Tokens per second')}
                  data-testid="query-tokens-per-second"
                >
                  <Gauge className="h-3 w-3" aria-hidden="true" />
                  {tokensPerSecond}/s
                </span>
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">
                  {t('query.tokensPerSecondDesc', 'Generation speed')}: {tokensPerSecond}{' '}
                  {t('query.tokensPerSecondUnit', 'tokens/second')}
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
        {modeMeta && ModeIcon && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <span
                  className={cn(
                    'flex items-center gap-1',
                    modeMeta.color,
                  )}
                  data-testid="query-response-mode"
                >
                  <ModeIcon className="h-3 w-3" aria-hidden="true" />
                  {modeMeta.label}
                </span>
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs max-w-xs">
                  {t('query.modeUsed', 'Mode')}: {modeMeta.label}
                  <br />
                  {modeMeta.description}
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
        {modelLabel && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <span
                  className="flex items-center gap-1 max-w-[14rem] truncate"
                  data-testid="query-response-model"
                >
                  <Brain className="h-3 w-3 shrink-0" aria-hidden="true" />
                  <span className="truncate">{modelLabel}</span>
                </span>
              </TooltipTrigger>
              <TooltipContent>
                <p className="text-xs">
                  {t('query.llmLineage', 'LLM Provider')}: {llmProvider || 'server default'}
                  {llmModel && (
                    <>
                      <br />
                      {t('query.llmModel', 'Model')}: {llmModel}
                    </>
                  )}
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </div>

      {sessionId ? (
        <LangfuseOpenSessionLink sessionId={sessionId} className="h-7 text-xs px-2" />
      ) : null}

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
      role="status"
      aria-live="polite"
      aria-label={t('query.generating', 'Generating response...')}
    >
      <div className="flex items-center gap-2 text-muted-foreground">
        {/* Simple pulsing dot - no expanding ring */}
        <span className="inline-flex h-2 w-2 rounded-full bg-primary motion-safe:animate-pulse" aria-hidden="true" />
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
  const router = useRouter();
  const sessionId = useActiveConversationId();
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
  const sourceCount = message.context?.sources?.length ?? 0;
  const queryMode = message.mode ?? 'mix';
  const displayContent = displayQueryAnswer(
    parsed.response || message.content,
    sourceCount,
    queryMode,
  );
  const showZeroAuthzHelp =
    !message.isStreaming &&
    !message.isError &&
    queryMode !== 'bypass' &&
    isZeroAuthzAnswer(parsed.response || message.content, sourceCount);

  return (
    <div
      className="flex justify-start mb-6 group motion-safe:animate-slide-in-left"
      role="article"
      aria-label={t('query.assistantMessage', 'Assistant response')}
    >
      <div className="flex items-start gap-3 max-w-full min-w-0">
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
            <Sparkles className="h-4 w-4" aria-hidden="true" />
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
                    className=""
                  />
                  {showZeroAuthzHelp && (
                    <p
                      className="mt-2 text-sm text-muted-foreground"
                      data-testid="spec146-zero-authz-help"
                    >
                      {t('query.zeroAuthzHelp', ZERO_AUTHZ_HELP)}
                    </p>
                  )}
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
              sessionId={sessionId}
              copied={copied}
              onCopy={handleCopy}
              onRegenerate={onRegenerate}
              isLast={isLast}
              isVisible={!!isLast}
            />
          )}

          {/* Source Citations — SPEC-100: reserve region until context arrives (no null→tall CLS) */}
          {!message.isStreaming && displayContent && (
            <div
              className="mt-2 min-h-[7.5rem]"
              data-testid="spec100-query-citations-slot"
            >
              {message.context ? (
                <SourceCitations
                  context={message.context}
                  onEntityClick={(entityId) => {
                    // Use router.push so browser history is preserved (back-button works)
                    router.push(`/graph?entity=${encodeURIComponent(entityId)}`);
                  }}
                  onDocumentClick={(documentId, chunkContent, chunkIndex, startLine, endLine, chunkId, page) => {
                    const url = buildDocumentCitationUrl({
                      documentId: encodeURIComponent(documentId),
                      chunkId,
                      page,
                      chunkContent,
                      startLine,
                      endLine,
                    });

                    // router.push preserves browser history so the back-button returns here
                    router.push(url);
                  }}
                  onExploreGraph={(entityLabels) => {
                    const params = new URLSearchParams();
                    if (entityLabels.length > 0) {
                      params.set('entities', entityLabels.join(','));
                      params.set('focus', entityLabels[0]);
                    }
                    router.push(`/graph${params.toString() ? `?${params}` : ''}`);
                  }}
                />
              ) : null}
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
