'use client';

import { Avatar, AvatarFallback } from '@/components/ui/avatar';
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
import { Textarea } from '@/components/ui/textarea';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { query as queryApi, queryStream } from '@/lib/api/edgequake';
import {
    useActiveConversation,
    useConversationStore,
    type ConversationMessage,
} from '@/stores/use-conversation-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { QueryContext } from '@/types';
import { useMutation } from '@tanstack/react-query';
import {
    BookOpen,
    Brain,
    Check,
    ChevronDown,
    ChevronRight,
    Clock,
    Copy,
    Gauge,
    GitBranch,
    Info,
    Lightbulb,
    Plus,
    RefreshCw,
    Search,
    Send,
    Settings2,
    Sliders,
    Sparkles,
    StopCircle,
    Thermometer,
    User,
    Zap
} from 'lucide-react';
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { ConversationHistoryPanel } from './conversation-history-panel';
import { MarkdownRenderer } from './markdown-renderer';
import { QueryModeSelector } from './query-mode-selector';
import { SourceCitations } from './source-citations';
import { parseCOTContent } from './thinking-display';

// Streaming state for better UX
type StreamingState = 'idle' | 'thinking' | 'generating' | 'complete' | 'error';

// Use ConversationMessage from store for local Message type alias
type Message = ConversationMessage;

// ============================================================================
// Delightful Loading Indicator - Shows animated placeholder while waiting
// ============================================================================

const LoadingMessage = memo(function LoadingMessage() {
  const { t } = useTranslation();
  
  return (
    <div className="flex justify-start mb-4">
      <div className="flex items-start gap-3 max-w-[85%]">
        <Avatar className="h-8 w-8 shrink-0 mt-1">
          <AvatarFallback className="bg-gradient-to-br from-primary/80 to-primary">
            <Sparkles className="h-4 w-4 text-primary-foreground animate-pulse" />
          </AvatarFallback>
        </Avatar>

        <div className="space-y-3 min-w-0 flex-1">
          <div className="bg-card border rounded-2xl rounded-tl-sm px-4 py-4">
            <div className="flex items-center gap-3">
              {/* Animated brain icon */}
              <div className="relative">
                <Brain className="h-5 w-5 text-primary" />
                <span className="absolute -top-0.5 -right-0.5 flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary/60 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-primary" />
                </span>
              </div>
              
              {/* Loading text with pulse */}
              <span className="text-sm text-muted-foreground">
                {t('query.processing', 'Processing your query...')}
              </span>
              
              {/* Animated dots */}
              <div className="flex gap-1 ml-2">
                <span 
                  className="w-2 h-2 bg-primary rounded-full animate-bounce" 
                  style={{ animationDelay: '0ms', animationDuration: '0.6s' }} 
                />
                <span 
                  className="w-2 h-2 bg-primary/80 rounded-full animate-bounce" 
                  style={{ animationDelay: '150ms', animationDuration: '0.6s' }} 
                />
                <span 
                  className="w-2 h-2 bg-primary/60 rounded-full animate-bounce" 
                  style={{ animationDelay: '300ms', animationDuration: '0.6s' }} 
                />
              </div>
            </div>
            
            {/* Progress shimmer effect */}
            <div className="mt-3 space-y-2">
              <div className="h-3 bg-muted rounded-full overflow-hidden">
                <div className="h-full w-1/3 bg-gradient-to-r from-transparent via-foreground/5 to-transparent animate-shimmer" />
              </div>
              <div className="h-3 bg-muted rounded-full w-3/4 overflow-hidden">
                <div className="h-full w-1/3 bg-gradient-to-r from-transparent via-foreground/5 to-transparent animate-shimmer" style={{ animationDelay: '150ms' }} />
              </div>
              <div className="h-3 bg-muted rounded-full w-1/2 overflow-hidden">
                <div className="h-full w-1/3 bg-gradient-to-r from-transparent via-foreground/5 to-transparent animate-shimmer" style={{ animationDelay: '300ms' }} />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});

// ============================================================================
// Chat Message Component - SOTA UX with thinking states, copy, actions
// ============================================================================

const ChatMessage = memo(function ChatMessage({
  message,
  onCopy,
  onRegenerate,
  isLast,
}: {
  message: Message;
  onCopy?: () => void;
  onRegenerate?: () => void;
  isLast?: boolean;
}) {
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

  const parsed = parseCOTContent(message.content);
  const hasThinking = parsed.thinking.length > 0;
  const displayContent = parsed.response;

  if (message.role === 'user') {
    return (
      <div className="flex justify-end mb-6">
        <div className="flex items-start gap-3 max-w-[85%]">
          <div className="bg-primary text-primary-foreground rounded-2xl rounded-tr-sm px-4 py-3 shadow-sm">
            <p className="whitespace-pre-wrap break-words overflow-wrap-anywhere">{message.content}</p>
          </div>
          <Avatar className="h-8 w-8 shrink-0 ring-2 ring-background shadow-sm">
            <AvatarFallback className="bg-primary/10">
              <User className="h-4 w-4" />
            </AvatarFallback>
          </Avatar>
        </div>
      </div>
    );
  }

  // Assistant message
  return (
    <div className="flex justify-start mb-6 group">
      <div className="flex items-start gap-3 max-w-[85%] min-w-0">
        <Avatar className="h-8 w-8 shrink-0 mt-1 ring-2 ring-background shadow-sm">
          <AvatarFallback className="bg-gradient-to-br from-primary/80 to-primary">
            <Sparkles className="h-4 w-4 text-primary-foreground" />
          </AvatarFallback>
        </Avatar>

        <div className="space-y-2 min-w-0 flex-1">
          {/* Model name like OpenWebUI */}
          <div className="flex items-center gap-2 text-sm">
            <span className="font-medium text-foreground">EdgeQuake</span>
            {message.timestamp && (
              <span className="text-xs text-muted-foreground">
                {new Date(message.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
              </span>
            )}
          </div>
          {/* Thinking Section */}
          {hasThinking && (
            <div className="rounded-lg border border-muted bg-muted/30">
              <button
                onClick={() => setThinkingExpanded(!thinkingExpanded)}
                className="flex items-center gap-2 w-full p-3 text-left hover:bg-muted/50 transition-colors rounded-t-lg"
              >
                {thinkingExpanded ? (
                  <ChevronDown className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-4 w-4 text-muted-foreground" />
                )}
                <Brain className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm font-medium text-foreground/80">
                  {t('query.reasoning', 'Reasoning')}
                </span>
                {message.thinkingTimeMs && (
                  <span className="text-xs text-muted-foreground ml-auto flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {(message.thinkingTimeMs / 1000).toFixed(1)}s
                  </span>
                )}
              </button>
              {thinkingExpanded && (
                <div className="p-3 pt-0 border-t border-muted">
                  <div className="text-sm text-muted-foreground whitespace-pre-wrap pl-4 border-l-2 border-muted">
                    {parsed.thinking.join('\n\n')}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Main Response */}
          {(displayContent || message.isStreaming) && (
            <div className="bg-card border rounded-2xl rounded-tl-sm px-4 py-3 shadow-sm">
              {message.isError ? (
                <p className="text-destructive break-words overflow-wrap-anywhere">{displayContent}</p>
              ) : displayContent ? (
                <div className="break-words overflow-wrap-anywhere hyphens-auto">
                  <MarkdownRenderer
                    content={displayContent}
                    isStreaming={message.isStreaming}
                    className="break-words prose prose-sm max-w-none"
                  />
                </div>
              ) : null}
              {message.isStreaming && (
                <span className="inline-block w-2 h-4 bg-foreground animate-pulse ml-1" />
              )}
            </div>
          )}

          {/* Streaming indicator when in thinking phase */}
          {message.isStreaming && !displayContent && hasThinking && (
            <div className="bg-card border rounded-2xl rounded-tl-sm px-4 py-3 shadow-sm">
              <div className="flex items-center gap-2 text-muted-foreground">
                <div className="flex gap-1">
                  <span className="w-2 h-2 bg-primary rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                  <span className="w-2 h-2 bg-primary/80 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                  <span className="w-2 h-2 bg-primary/60 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
                </div>
                <span className="text-sm font-medium">{t('query.generating', 'Generating response...')}</span>
              </div>
            </div>
          )}

          {/* Metadata & Actions */}
          {!message.isStreaming && displayContent && (
            <div className={`flex items-center gap-2 transition-opacity ${isLast ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'}`}>
              {/* Stats */}
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                {message.mode && (
                  <Badge variant="outline" className="text-xs font-normal">
                    {message.mode}
                  </Badge>
                )}
                {message.tokensUsed && (
                  <span className="flex items-center gap-1">
                    <Zap className="h-3 w-3" />
                    {message.tokensUsed}
                  </span>
                )}
                {message.durationMs && (
                  <span className="flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {(message.durationMs / 1000).toFixed(1)}s
                  </span>
                )}
              </div>

              {/* Action buttons */}
              <div className="flex items-center gap-1 ml-auto">
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 hover:bg-muted"
                        onClick={handleCopy}
                      >
                        {copied ? (
                          <Check className="h-3.5 w-3.5 text-green-500" />
                        ) : (
                          <Copy className="h-3.5 w-3.5" />
                        )}
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent side="bottom">{t('common.copy', 'Copy')}</TooltipContent>
                  </Tooltip>
                </TooltipProvider>

                {isLast && onRegenerate && (
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7 hover:bg-muted"
                          onClick={onRegenerate}
                        >
                          <RefreshCw className="h-3.5 w-3.5" />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent side="bottom">{t('query.regenerate', 'Regenerate')}</TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                )}
              </div>
            </div>
          )}

          {/* Source Citations */}
          {message.context && !message.isStreaming && (
            <div className="mt-2">
              <SourceCitations
                context={message.context}
                onEntityClick={(entityId) => {
                  window.location.href = `/graph?entity=${encodeURIComponent(entityId)}`;
                }}
                onDocumentClick={(documentId) => {
                  window.location.href = `/documents?id=${encodeURIComponent(documentId)}`;
                }}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
});

// ============================================================================
// Empty State with suggestions and graph stats
// ============================================================================

interface EmptyStateProps {
  onSuggestionClick?: (text: string) => void;
  graphStats?: { entities: number; relationships: number; types: number };
}

const EmptyState = memo(function EmptyState({ onSuggestionClick, graphStats }: EmptyStateProps) {
  const { t } = useTranslation();

  const suggestions = [
    {
      icon: <Search className="h-4 w-4" />,
      text: t('query.suggestions.0', 'What are the main entities in my knowledge graph?'),
      category: 'exploration',
    },
    {
      icon: <Lightbulb className="h-4 w-4" />,
      text: t('query.suggestions.1', 'Summarize the key relationships between documents'),
      category: 'summary',
    },
    {
      icon: <GitBranch className="h-4 w-4" />,
      text: t('query.suggestions.2', 'Find connections between people and organizations'),
      category: 'relationships',
    },
    {
      icon: <BookOpen className="h-4 w-4" />,
      text: t('query.suggestions.3', 'What topics are covered in my documents?'),
      category: 'topics',
    },
  ];

  const hasData = graphStats && (graphStats.entities > 0 || graphStats.relationships > 0);

  return (
    <div className="flex flex-col items-center justify-center h-full py-12 px-4 animate-fade-in-up">
      {/* Animated icon */}
      <div className="relative mb-8">
        <div className="absolute inset-0 bg-gradient-to-r from-primary/40 to-primary/60 rounded-2xl blur-2xl opacity-20 animate-pulse-soft" />
        <div className="relative bg-gradient-to-br from-primary/80 to-primary rounded-2xl p-5 shadow-lg">
          <Sparkles className="h-10 w-10 text-primary-foreground" />
        </div>
      </div>
      
      {/* Title and description */}
      <h2 className="text-2xl font-bold mb-2 text-center">
        {t('query.emptyTitle', 'Ask about your knowledge graph')}
      </h2>
      <p className="text-muted-foreground text-center mb-8 max-w-lg leading-relaxed">
        {t('query.emptyDescription', 'I can help you explore entities, find connections, and uncover insights from your documents.')}
      </p>

      {/* Graph stats (if available) */}
      {hasData && (
        <div className="flex items-center gap-4 mb-8 px-6 py-3 bg-muted/30 rounded-full border border-border/50">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-green-500" />
            <span className="text-sm font-medium">{graphStats.entities}</span>
            <span className="text-xs text-muted-foreground">entities</span>
          </div>
          <div className="w-px h-4 bg-border" />
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-amber-500" />
            <span className="text-sm font-medium">{graphStats.relationships}</span>
            <span className="text-xs text-muted-foreground">relationships</span>
          </div>
          <div className="w-px h-4 bg-border" />
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-blue-500" />
            <span className="text-sm font-medium">{graphStats.types}</span>
            <span className="text-xs text-muted-foreground">types</span>
          </div>
        </div>
      )}

      {/* Suggestions */}
      {onSuggestionClick && (
        <div className="w-full max-w-2xl space-y-3">
          <p className="text-sm font-medium text-muted-foreground text-center mb-3">
            {t('query.tryAsking', 'Try asking:')}
          </p>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
            {suggestions.map((suggestion, i) => (
              <button
                key={i}
                onClick={() => onSuggestionClick(suggestion.text)}
                className="group flex items-start gap-3 text-left px-4 py-3.5 rounded-xl border bg-card hover:bg-muted/50 hover:border-primary/30 transition-all duration-200 hover:shadow-sm hover:-translate-y-0.5"
              >
                <div className="p-1.5 rounded-lg bg-muted group-hover:bg-primary/10 transition-colors shrink-0">
                  {suggestion.icon}
                </div>
                <span className="text-sm leading-relaxed">{suggestion.text}</span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
});

// ============================================================================
// Main Query Interface Component
// ============================================================================

export function QueryInterface() {
  const { t } = useTranslation();
  const [input, setInput] = useState('');
  const [streamingState, setStreamingState] = useState<StreamingState>('idle');
  const [shouldAutoScroll, setShouldAutoScroll] = useState(true);
  const scrollRef = useRef<HTMLDivElement>(null);
  const scrollAnchorRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const thinkingStartRef = useRef<number | null>(null);

  const { querySettings, setQuerySettings } = useSettingsStore();
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  
  // Use the new conversation store
  const {
    activeConversationId,
    createConversation,
    addMessage,
    updateMessage,
    autoTitleConversation,
    clearActiveConversation,
  } = useConversationStore();
  
  const activeConversation = useActiveConversation();
  
  // Memoize messages to avoid dependency issues with hooks
  const messages = useMemo(() => activeConversation?.messages ?? [], [activeConversation?.messages]);

  // Wrapper to maintain API compatibility
  const setMessages = useCallback((msgs: Message[]) => {
    // For setMessages, we need to clear and re-add
    // This is less efficient but maintains compatibility
    clearActiveConversation();
    msgs.forEach(m => addMessage(m));
  }, [clearActiveConversation, addMessage]);

  // Handle tenant/workspace change - create a new conversation
  useEffect(() => {
    // Only handle context change if there are messages
    if (messages.length > 0) {
      // Create a new conversation for the new context
      createConversation(selectedTenantId ?? undefined, selectedWorkspaceId ?? undefined);
      toast(t('query.conversationCleared', 'New conversation started'), {
        description: t('query.conversationClearedDesc', 'Context has changed. Starting a fresh conversation.'),
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedTenantId, selectedWorkspaceId]);

  // Smart scroll to bottom when messages change - only if user hasn't scrolled up
  useEffect(() => {
    if (!shouldAutoScroll) return;
    
    if (scrollAnchorRef.current) {
      scrollAnchorRef.current.scrollIntoView({ behavior: 'smooth', block: 'end' });
    }
  }, [messages, streamingState, shouldAutoScroll]);

  // Detect if user has scrolled up (to disable auto-scroll)
  useEffect(() => {
    const viewport = scrollRef.current?.querySelector('[data-radix-scroll-area-viewport]');
    if (!viewport) return;

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = viewport as HTMLElement;
      // If user is near the bottom (within 100px), enable auto-scroll
      const isNearBottom = scrollHeight - scrollTop - clientHeight < 100;
      setShouldAutoScroll(isNearBottom);
    };

    viewport.addEventListener('scroll', handleScroll);
    return () => viewport.removeEventListener('scroll', handleScroll);
  }, []);

  // Re-enable auto-scroll when streaming starts
  useEffect(() => {
    if (streamingState === 'thinking' || streamingState === 'generating') {
      setShouldAutoScroll(true);
    }
  }, [streamingState]);

  // Auto-resize textarea
  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value);
    e.target.style.height = 'auto';
    e.target.style.height = Math.min(e.target.scrollHeight, 200) + 'px';
  }, []);

  // Stop generation
  const handleStop = useCallback(() => {
    abortControllerRef.current?.abort();
    setStreamingState('idle');
  }, []);

  const handleStreamQuery = useCallback(async (queryText: string) => {
    const messageId = crypto.randomUUID();
    setStreamingState('thinking');
    thinkingStartRef.current = Date.now();
    abortControllerRef.current = new AbortController();

    // Add placeholder message using store action
    const assistantMessage: Message = {
      id: messageId,
      role: 'assistant',
      content: '',
      mode: querySettings.mode,
      isStreaming: true,
      timestamp: Date.now(),
    };
    addMessage(assistantMessage);

    try {
      let fullContent = '';
      let tokensUsed = 0;
      let durationMs = 0;
      let context: QueryContext | undefined;
      let thinkingTimeMs: number | undefined;

      for await (const chunk of queryStream({
        query: queryText,
        mode: querySettings.mode,
        top_k: querySettings.topK,
        max_tokens: querySettings.maxTokens,
        temperature: querySettings.temperature,
        stream: true,
      })) {
        if (abortControllerRef.current?.signal.aborted) {
          break;
        }

        if (chunk.type === 'token' && chunk.content) {
          fullContent += chunk.content;

          // Check if we transitioned from thinking to generating
          const parsed = parseCOTContent(fullContent);
          if (parsed.response && !thinkingTimeMs && thinkingStartRef.current) {
            thinkingTimeMs = Date.now() - thinkingStartRef.current;
            setStreamingState('generating');
          }

          updateMessage(messageId, { content: fullContent, thinkingTimeMs });
        } else if (chunk.type === 'context' && chunk.context) {
          context = chunk.context;
        } else if (chunk.type === 'done') {
          tokensUsed = chunk.tokens_used || 0;
          durationMs = chunk.duration_ms || 0;
        } else if (chunk.type === 'error') {
          throw new Error(chunk.error || 'Streaming failed');
        }
      }

      // Finalize message using store action
      updateMessage(messageId, {
        content: fullContent,
        isStreaming: false,
        tokensUsed,
        durationMs,
        thinkingTimeMs,
        context,
      });

      // Auto-title the conversation after first exchange
      if (activeConversationId && messages.length <= 1) {
        autoTitleConversation(activeConversationId);
      }

      setStreamingState('complete');
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        setStreamingState('idle');
        return;
      }

      const errorMessage = error instanceof Error ? error.message : 'Query failed';
      toast.error(errorMessage, {
        action: {
          label: t('common.retry', 'Retry'),
          onClick: () => {
            // User can retry by resubmitting the same query
          },
        },
      });

      // Update error message using store action
      updateMessage(messageId, { content: errorMessage, isStreaming: false, isError: true });

      setStreamingState('error');
    } finally {
      abortControllerRef.current = null;
      thinkingStartRef.current = null;
    }
  }, [querySettings, addMessage, updateMessage, activeConversationId, messages.length, autoTitleConversation, t]);

  const queryMutation = useMutation({
    mutationFn: async (queryText: string) => {
      return queryApi({
        query: queryText,
        mode: querySettings.mode,
        top_k: querySettings.topK,
        max_tokens: querySettings.maxTokens,
        temperature: querySettings.temperature,
        stream: false,
      });
    },
    onSuccess: (data) => {
      // Add assistant response using store action
      addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: data.answer,
        mode: data.mode,
        tokensUsed: data.tokens_used,
        durationMs: data.duration_ms,
        context: data.context,
        timestamp: Date.now(),
      });

      // Auto-title the conversation after first exchange
      if (activeConversationId && messages.length <= 1) {
        autoTitleConversation(activeConversationId);
      }
    },
    onError: (error) => {
      toast.error(t('query.failed', 'Query failed'), {
        description: error instanceof Error ? error.message : t('common.unknownError', 'Unknown error'),
        action: {
          label: t('common.retry', 'Retry'),
          onClick: () => {
            // User can retry from the UI
          },
        },
      });
    },
  });

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    if (!input.trim() || streamingState !== 'idle' && streamingState !== 'complete' && streamingState !== 'error') return;
    if (queryMutation.isPending) return;

    const queryText = input.trim();
    setInput('');
    
    // Reset textarea height
    if (inputRef.current) {
      inputRef.current.style.height = 'auto';
    }

    // Create a new conversation if none exists
    if (!activeConversationId) {
      createConversation(selectedTenantId ?? undefined, selectedWorkspaceId ?? undefined);
    }

    // Add user message using store action
    addMessage({
      id: crypto.randomUUID(),
      role: 'user',
      content: queryText,
      timestamp: Date.now(),
    });

    // Use streaming or regular query
    if (querySettings.stream) {
      await handleStreamQuery(queryText);
    } else {
      queryMutation.mutate(queryText);
    }
  };

  // Handle regenerate
  const handleRegenerate = useCallback(() => {
    if (messages.length < 2) return;
    const lastUserMessage = [...messages].reverse().find((m) => m.role === 'user');
    if (!lastUserMessage) return;

    // Remove last assistant message
    const filteredMessages = messages.slice(0, -1);
    setMessages(filteredMessages);

    // Defer the regeneration to next tick to ensure state is updated
    // This prevents race conditions between state update and new message creation
    setTimeout(() => {
      handleStreamQuery(lastUserMessage.content);
    }, 0);
  }, [messages, setMessages, handleStreamQuery]);

  // Handle suggestion click
  const handleSuggestionClick = useCallback((text: string) => {
    setInput(text);
    inputRef.current?.focus();
  }, []);

  // Handle new conversation
  const handleNewConversation = useCallback(() => {
    createConversation(selectedTenantId ?? undefined, selectedWorkspaceId ?? undefined);
    setInput('');
    setStreamingState('idle');
  }, [createConversation, selectedTenantId, selectedWorkspaceId]);

  const isLoading = streamingState === 'thinking' || streamingState === 'generating' || queryMutation.isPending;

  return (
    <div className="flex h-full min-h-0">
      {/* Main Query Area */}
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {/* Header */}
        <header className="flex items-center justify-between border-b px-5 py-3 shrink-0 bg-background/80 backdrop-blur-sm">
          <div className="flex items-center gap-3">
            <h1 className="text-lg font-semibold tracking-tight">{t('query.title', 'Query')}</h1>
            <span className="text-xs text-muted-foreground hidden sm:inline">
              {t('query.subtitle', 'Ask questions about your knowledge graph')}
            </span>
          </div>
          <div className="flex items-center gap-3">
            {/* New Conversation Button */}
            <Button
              variant="outline"
              size="sm"
              onClick={handleNewConversation}
              disabled={isLoading}
              className="gap-1"
            >
              <Plus className="h-4 w-4" />
              {t('query.newConversation', 'New')}
            </Button>

            {/* Mode Selector */}
            <QueryModeSelector
              value={querySettings.mode}
              onChange={(mode) => setQuerySettings({ mode })}
              disabled={isLoading}
            />

            {/* Settings */}
            <Sheet>
              <SheetTrigger asChild>
                <Button variant="ghost" size="icon">
                  <Settings2 className="h-4 w-4" />
                </Button>
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
                      <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t('query.settings.responseMode', 'Response Mode')}</h3>
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
                          checked={querySettings.stream}
                          onCheckedChange={(stream) => setQuerySettings({ stream })}
                        />
                      </div>
                    </div>
                  </div>

                  <Separator />

                  {/* Retrieval Section */}
                  <div className="space-y-3">
                    <div className="flex items-center gap-2">
                      <BookOpen className="h-3.5 w-3.5 text-blue-500" />
                      <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t('query.settings.retrieval', 'Retrieval')}</h3>
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
                                  <p className="text-xs">{t('query.settings.topKHint', 'Number of relevant chunks to retrieve from the knowledge graph')}</p>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          </div>
                          <Badge variant="secondary" className="font-mono text-[10px] h-5 px-1.5">
                            {querySettings.topK}
                          </Badge>
                        </div>
                        <Slider
                          value={[querySettings.topK]}
                          onValueChange={([topK]) => setQuerySettings({ topK })}
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
                      <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t('query.settings.generation', 'Generation')}</h3>
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
                                  <p className="text-xs">{t('query.settings.temperatureHint', 'Controls randomness. Lower = more focused, higher = more creative')}</p>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          </div>
                          <Badge variant="secondary" className="font-mono text-[10px] h-5 px-1.5">
                            {querySettings.temperature.toFixed(1)}
                          </Badge>
                        </div>
                        <Slider
                          value={[querySettings.temperature]}
                          onValueChange={([temperature]) => setQuerySettings({ temperature })}
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
                                  <p className="text-xs">{t('query.settings.maxTokensHint', 'Maximum length of the generated response')}</p>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          </div>
                          <Badge variant="secondary" className="font-mono text-[10px] h-5 px-1.5">
                            {querySettings.maxTokens}
                          </Badge>
                        </div>
                        <Slider
                          value={[querySettings.maxTokens]}
                          onValueChange={([maxTokens]) => setQuerySettings({ maxTokens })}
                          min={256}
                          max={4096}
                          step={256}
                          className="w-full"
                        />
                        <div className="flex justify-between text-[10px] text-muted-foreground">
                          <span>256 (Short)</span>
                          <span>4096 (Long)</span>
                        </div>
                      </div>
                    </div>
                  </div>
                  </div>
                </ScrollArea>
              </SheetContent>
            </Sheet>
          </div>
        </header>

        {/* Messages - improved padding */}
        <div className="flex-1 min-h-0 overflow-hidden">
          <ScrollArea ref={scrollRef} className="h-full">
            <div className="max-w-3xl mx-auto px-6 py-6">
              {messages.length === 0 && !queryMutation.isPending ? (
                <EmptyState onSuggestionClick={handleSuggestionClick} />
              ) : (
                <>
                  {messages.map((message, index) => (
                    <ChatMessage
                      key={message.id}
                      message={message}
                      onRegenerate={
                        message.role === 'assistant' && index === messages.length - 1
                          ? handleRegenerate
                          : undefined
                      }
                      isLast={index === messages.length - 1}
                    />
                  ))}
                  {/* Show loading message for non-streaming queries */}
                  {queryMutation.isPending && <LoadingMessage />}
                </>
              )}
              {/* Scroll anchor for auto-scroll */}
              <div ref={scrollAnchorRef} className="h-6" />
            </div>
          </ScrollArea>
        </div>

        {/* Input - Fixed at bottom with improved spacing */}
        <div className="border-t px-6 py-4 bg-background flex-shrink-0" role="form" aria-label={t('query.form', 'Query form')}>
          <form onSubmit={handleSubmit} className="max-w-3xl mx-auto">
            <div className="relative">
              <Textarea
                ref={inputRef}
                value={input}
                onChange={handleInputChange}
                placeholder={t('query.placeholder', 'Ask a question...')}
                className="min-h-[56px] max-h-[200px] resize-none pr-24 py-4 text-base query-input focus:ring-2 focus:ring-primary/30 focus:border-primary transition-all duration-200"
                rows={1}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    handleSubmit();
                  }
                }}
                disabled={isLoading}
                aria-label={t('query.placeholder', 'Ask a question')}
                aria-describedby="query-hint"
              />
              <span id="query-hint" className="sr-only">
                Press Enter to send, Shift+Enter for new line
              </span>
              <div className="absolute right-3 bottom-3 flex items-center gap-2">
                {isLoading ? (
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={handleStop}
                    className="h-9"
                    aria-label={t('query.stop', 'Stop generating')}
                  >
                    <StopCircle className="h-4 w-4 mr-1" aria-hidden="true" />
                    Stop
                  </Button>
                ) : (
                  <Button
                    type="submit"
                    size="sm"
                    disabled={!input.trim()}
                    className="h-8"
                    aria-label={t('query.submit', 'Send message')}
                  >
                    <Send className="h-4 w-4" aria-hidden="true" />
                  </Button>
                )}
              </div>
            </div>
            <p className="text-xs text-muted-foreground mt-2 text-center" aria-hidden="true">
              {t('query.hint', 'Press Enter to send, Shift+Enter for new line')}
            </p>
          </form>
        </div>
      </div>

      {/* Conversation History Panel - New collapsible component */}
      <ConversationHistoryPanel />
    </div>
  );
}

export default QueryInterface;
