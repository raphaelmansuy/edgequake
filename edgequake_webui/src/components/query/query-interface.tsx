'use client';

import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
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
import { useFavoriteQueries, useQueryStore, useRecentQueries, type ChatMessage } from '@/stores/use-query-store';
import { useSettingsStore } from '@/stores/use-settings-store';
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
    History,
    Info,
    Loader2,
    MessageSquare,
    RefreshCw,
    Send,
    Settings2,
    Sliders,
    Sparkles,
    Star,
    StopCircle,
    Thermometer,
    Trash2,
    User,
    Zap,
} from 'lucide-react';
import { memo, useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { MarkdownRenderer } from './markdown-renderer';
import { QueryModeSelector } from './query-mode-selector';
import { SourceCitations } from './source-citations';
import { parseCOTContent } from './thinking-display';

// Streaming state for better UX
type StreamingState = 'idle' | 'thinking' | 'generating' | 'complete' | 'error';

// Use ChatMessage from store for local Message type alias
type Message = ChatMessage;

// ============================================================================
// Delightful Loading Indicator - Shows animated placeholder while waiting
// ============================================================================

const LoadingMessage = memo(function LoadingMessage() {
  const { t } = useTranslation();
  
  return (
    <div className="flex justify-start mb-4">
      <div className="flex items-start gap-3 max-w-[85%]">
        <Avatar className="h-8 w-8 shrink-0 mt-1">
          <AvatarFallback className="bg-gradient-to-br from-violet-500 to-purple-600">
            <Sparkles className="h-4 w-4 text-white animate-pulse" />
          </AvatarFallback>
        </Avatar>

        <div className="space-y-3 min-w-0 flex-1">
          <div className="bg-card border rounded-2xl rounded-tl-sm px-4 py-4">
            <div className="flex items-center gap-3">
              {/* Animated brain icon */}
              <div className="relative">
                <Brain className="h-5 w-5 text-purple-500" />
                <span className="absolute -top-0.5 -right-0.5 flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-purple-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-purple-500" />
                </span>
              </div>
              
              {/* Loading text with pulse */}
              <span className="text-sm text-muted-foreground">
                {t('query.processing', 'Processing your query...')}
              </span>
              
              {/* Animated dots */}
              <div className="flex gap-1 ml-2">
                <span 
                  className="w-2 h-2 bg-purple-500 rounded-full animate-bounce" 
                  style={{ animationDelay: '0ms', animationDuration: '0.6s' }} 
                />
                <span 
                  className="w-2 h-2 bg-purple-400 rounded-full animate-bounce" 
                  style={{ animationDelay: '150ms', animationDuration: '0.6s' }} 
                />
                <span 
                  className="w-2 h-2 bg-purple-300 rounded-full animate-bounce" 
                  style={{ animationDelay: '300ms', animationDuration: '0.6s' }} 
                />
              </div>
            </div>
            
            {/* Progress shimmer effect */}
            <div className="mt-3 space-y-2">
              <div className="h-3 bg-gradient-to-r from-muted via-muted-foreground/10 to-muted rounded-full overflow-hidden">
                <div className="h-full w-1/3 bg-gradient-to-r from-transparent via-purple-200/40 to-transparent animate-shimmer" />
              </div>
              <div className="h-3 bg-gradient-to-r from-muted via-muted-foreground/10 to-muted rounded-full w-3/4 overflow-hidden">
                <div className="h-full w-1/3 bg-gradient-to-r from-transparent via-purple-200/40 to-transparent animate-shimmer" style={{ animationDelay: '150ms' }} />
              </div>
              <div className="h-3 bg-gradient-to-r from-muted via-muted-foreground/10 to-muted rounded-full w-1/2 overflow-hidden">
                <div className="h-full w-1/3 bg-gradient-to-r from-transparent via-purple-200/40 to-transparent animate-shimmer" style={{ animationDelay: '300ms' }} />
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
            <p className="whitespace-pre-wrap">{message.content}</p>
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
          <AvatarFallback className="bg-gradient-to-br from-violet-500 to-purple-600">
            <Sparkles className="h-4 w-4 text-white" />
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
            <div className="rounded-lg border border-purple-200 dark:border-purple-800 bg-purple-50/50 dark:bg-purple-950/30">
              <button
                onClick={() => setThinkingExpanded(!thinkingExpanded)}
                className="flex items-center gap-2 w-full p-3 text-left hover:bg-purple-100/50 dark:hover:bg-purple-900/30 transition-colors rounded-t-lg"
              >
                {thinkingExpanded ? (
                  <ChevronDown className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                ) : (
                  <ChevronRight className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                )}
                <Brain className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                <span className="text-sm font-medium text-purple-700 dark:text-purple-300">
                  {t('query.reasoning', 'Reasoning')}
                </span>
                {message.thinkingTimeMs && (
                  <span className="text-xs text-purple-500 dark:text-purple-400 ml-auto flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {(message.thinkingTimeMs / 1000).toFixed(1)}s
                  </span>
                )}
              </button>
              {thinkingExpanded && (
                <div className="p-3 pt-0 border-t border-purple-200/50 dark:border-purple-800/50">
                  <div className="text-sm text-purple-800 dark:text-purple-200 whitespace-pre-wrap pl-4 border-l-2 border-purple-300 dark:border-purple-700">
                    {parsed.thinking.join('\n\n')}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Main Response */}
          {(displayContent || message.isStreaming) && (
            <div className="bg-card border rounded-2xl rounded-tl-sm px-4 py-3">
              {message.isError ? (
                <p className="text-destructive">{displayContent}</p>
              ) : displayContent ? (
                <MarkdownRenderer
                  content={displayContent}
                  isStreaming={message.isStreaming}
                />
              ) : null}
              {message.isStreaming && (
                <span className="inline-block w-2 h-4 bg-foreground animate-pulse ml-1" />
              )}
            </div>
          )}

          {/* Streaming indicator when in thinking phase */}
          {message.isStreaming && !displayContent && hasThinking && (
            <div className="bg-card border rounded-2xl rounded-tl-sm px-4 py-3">
              <div className="flex items-center gap-2 text-muted-foreground">
                <div className="flex gap-1">
                  <span className="w-2 h-2 bg-purple-500 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                  <span className="w-2 h-2 bg-purple-500 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                  <span className="w-2 h-2 bg-purple-500 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
                </div>
                <span className="text-sm">{t('query.generating', 'Generating response...')}</span>
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
// Typing Indicator - Shows thinking/generating state
// ============================================================================

const TypingIndicator = memo(function TypingIndicator({ state }: { state: StreamingState }) {
  const { t } = useTranslation();

  if (state === 'idle' || state === 'complete') return null;

  const messages: Record<StreamingState, string> = {
    thinking: t('query.thinking', 'Thinking...'),
    generating: t('query.generating', 'Generating response...'),
    error: t('query.error', 'An error occurred'),
    idle: '',
    complete: '',
  };

  return (
    <div className="flex justify-start mb-4">
      <div className="flex items-start gap-3">
        <Avatar className="h-8 w-8 shrink-0">
          <AvatarFallback className="bg-gradient-to-br from-violet-500 to-purple-600">
            <Sparkles className="h-4 w-4 text-white" />
          </AvatarFallback>
        </Avatar>
        <div className="bg-card border rounded-2xl rounded-tl-sm px-4 py-3">
          <div className="flex items-center gap-3">
            {state === 'thinking' && (
              <Brain className="h-4 w-4 text-purple-500 animate-pulse" />
            )}
            {state === 'generating' && (
              <Loader2 className="h-4 w-4 animate-spin text-primary" />
            )}
            <span className="text-sm text-muted-foreground">{messages[state]}</span>
            <div className="flex gap-1">
              <span className="w-1.5 h-1.5 bg-muted-foreground rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
              <span className="w-1.5 h-1.5 bg-muted-foreground rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
              <span className="w-1.5 h-1.5 bg-muted-foreground rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});

// ============================================================================
// Empty State with suggestions
// ============================================================================

const EmptyState = memo(function EmptyState({ onSuggestionClick }: { onSuggestionClick?: (text: string) => void }) {
  const { t } = useTranslation();

  const suggestions = [
    t('query.suggestions.0', 'What are the main entities in my knowledge graph?'),
    t('query.suggestions.1', 'Summarize the key relationships between documents'),
    t('query.suggestions.2', 'Find connections between people and organizations'),
    t('query.suggestions.3', 'What topics are covered in my documents?'),
  ];

  return (
    <div className="flex flex-col items-center justify-center h-full py-12 px-4">
      <div className="relative mb-6">
        <div className="absolute inset-0 bg-gradient-to-r from-violet-500 to-purple-600 rounded-full blur-xl opacity-20 animate-pulse" />
        <div className="relative bg-gradient-to-br from-violet-500 to-purple-600 rounded-full p-4">
          <MessageSquare className="h-8 w-8 text-white" />
        </div>
      </div>
      
      <h2 className="text-xl font-semibold mb-2">{t('query.emptyTitle', 'Start a conversation')}</h2>
      <p className="text-muted-foreground text-center mb-6 max-w-md">
        {t('query.emptyDescription', 'Ask questions about your knowledge graph and documents. I can help you explore relationships and find insights.')}
      </p>

      {onSuggestionClick && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-2 max-w-2xl w-full">
          {suggestions.map((suggestion, i) => (
            <button
              key={i}
              onClick={() => onSuggestionClick(suggestion)}
              className="text-left px-4 py-3 rounded-lg border bg-card hover:bg-accent transition-colors text-sm"
            >
              <span className="text-muted-foreground">→</span> {suggestion}
            </button>
          ))}
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
  const [currentStreamingId, setCurrentStreamingId] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const thinkingStartRef = useRef<number | null>(null);

  const { querySettings, setQuerySettings } = useSettingsStore();
  const { 
    addToHistory, 
    toggleFavorite, 
    removeFromHistory,
    conversationMessages: messages,
    setConversationMessages: setMessages,
    addConversationMessage,
    updateConversationMessage,
    clearConversation,
  } = useQueryStore();
  const recentQueries = useRecentQueries(10);
  const favoriteQueries = useFavoriteQueries();

  // Scroll to bottom when messages change
  useEffect(() => {
    if (scrollRef.current) {
      const viewport = scrollRef.current.querySelector('[data-radix-scroll-area-viewport]');
      if (viewport) {
        viewport.scrollTop = viewport.scrollHeight;
      }
    }
  }, [messages, streamingState]);

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

  const handleStreamQuery = async (queryText: string) => {
    const messageId = crypto.randomUUID();
    setCurrentStreamingId(messageId);
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
    addConversationMessage(assistantMessage);

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

          updateConversationMessage(messageId, { content: fullContent, thinkingTimeMs });
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
      updateConversationMessage(messageId, {
        content: fullContent,
        isStreaming: false,
        tokensUsed,
        durationMs,
        thinkingTimeMs,
        context,
      });

      addToHistory({
        query: queryText,
        mode: querySettings.mode,
        response: fullContent,
        isFavorite: false,
      });

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
      updateConversationMessage(messageId, { content: errorMessage, isStreaming: false, isError: true });

      setStreamingState('error');
    } finally {
      setCurrentStreamingId(null);
      abortControllerRef.current = null;
      thinkingStartRef.current = null;
    }
  };

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
    onSuccess: (data, queryText) => {
      // Add assistant response using store action
      addConversationMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: data.answer,
        mode: data.mode,
        tokensUsed: data.tokens_used,
        durationMs: data.duration_ms,
        context: data.context,
        timestamp: Date.now(),
      });

      addToHistory({
        query: queryText,
        mode: data.mode,
        response: data.answer,
        isFavorite: false,
      });
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

    // Add user message using store action
    addConversationMessage({
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
  }, [messages, setMessages, querySettings]);

  // Handle suggestion click
  const handleSuggestionClick = useCallback((text: string) => {
    setInput(text);
    inputRef.current?.focus();
  }, []);

  const handleHistoryClick = (query: string) => {
    setInput(query);
    inputRef.current?.focus();
  };

  const isLoading = streamingState === 'thinking' || streamingState === 'generating' || queryMutation.isPending;

  return (
    <div className="flex h-full">
      {/* Main Query Area */}
      <div className="flex-1 flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between border-b px-4 py-3">
          <div>
            <h1 className="text-lg font-semibold">{t('query.title', 'Query')}</h1>
            <p className="text-sm text-muted-foreground">
              {t('query.subtitle', 'Ask questions about your knowledge graph')}
            </p>
          </div>
          <div className="flex items-center gap-2">
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
              <SheetContent className="w-[400px] sm:w-[540px] overflow-y-auto">
                <SheetHeader className="pb-2">
                  <SheetTitle className="flex items-center gap-2">
                    <Sliders className="h-5 w-5 text-primary" />
                    {t('query.settings.title', 'Query Settings')}
                  </SheetTitle>
                  <SheetDescription>
                    {t('query.settings.description', 'Configure how the AI processes and responds to your queries.')}
                  </SheetDescription>
                </SheetHeader>
                
                <div className="mt-4 space-y-5 pb-6">
                  {/* Response Mode Section */}
                  <div className="space-y-4">
                    <div className="flex items-center gap-2">
                      <Zap className="h-4 w-4 text-amber-500" />
                      <h3 className="text-sm font-semibold">{t('query.settings.responseMode', 'Response Mode')}</h3>
                    </div>
                    
                    <div className="rounded-lg border p-4 space-y-4 bg-muted/30">
                      {/* Stream Toggle */}
                      <div className="flex items-center justify-between">
                        <div className="space-y-0.5">
                          <Label htmlFor="stream-toggle" className="text-sm font-medium cursor-pointer">
                            {t('query.settings.streaming', 'Streaming')}
                          </Label>
                          <p className="text-xs text-muted-foreground">
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
                  <div className="space-y-4">
                    <div className="flex items-center gap-2">
                      <BookOpen className="h-4 w-4 text-blue-500" />
                      <h3 className="text-sm font-semibold">{t('query.settings.retrieval', 'Retrieval')}</h3>
                    </div>
                    
                    <div className="rounded-lg border p-4 space-y-4 bg-muted/30">
                      {/* Top K */}
                      <div className="space-y-3">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <Label className="text-sm font-medium">
                              {t('query.settings.topK', 'Top K Results')}
                            </Label>
                            <TooltipProvider>
                              <Tooltip>
                                <TooltipTrigger>
                                  <Info className="h-3.5 w-3.5 text-muted-foreground" />
                                </TooltipTrigger>
                                <TooltipContent side="top" className="max-w-[200px]">
                                  <p className="text-xs">{t('query.settings.topKHint', 'Number of relevant chunks to retrieve from the knowledge graph')}</p>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          </div>
                          <Badge variant="secondary" className="font-mono text-xs">
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
                        <div className="flex justify-between text-xs text-muted-foreground">
                          <span>1 (Precise)</span>
                          <span>50 (Comprehensive)</span>
                        </div>
                      </div>
                    </div>
                  </div>

                  <Separator />

                  {/* Generation Section */}
                  <div className="space-y-4">
                    <div className="flex items-center gap-2">
                      <Brain className="h-4 w-4 text-purple-500" />
                      <h3 className="text-sm font-semibold">{t('query.settings.generation', 'Generation')}</h3>
                    </div>
                    
                    <div className="rounded-lg border p-4 space-y-5 bg-muted/30">
                      {/* Temperature */}
                      <div className="space-y-3">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <Thermometer className="h-3.5 w-3.5 text-muted-foreground" />
                            <Label className="text-sm font-medium">
                              {t('query.settings.temperature', 'Temperature')}
                            </Label>
                            <TooltipProvider>
                              <Tooltip>
                                <TooltipTrigger>
                                  <Info className="h-3.5 w-3.5 text-muted-foreground" />
                                </TooltipTrigger>
                                <TooltipContent side="top" className="max-w-[200px]">
                                  <p className="text-xs">{t('query.settings.temperatureHint', 'Controls randomness. Lower = more focused, higher = more creative')}</p>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          </div>
                          <Badge variant="secondary" className="font-mono text-xs">
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
                        <div className="flex justify-between text-xs text-muted-foreground">
                          <span>0 (Precise)</span>
                          <span>2 (Creative)</span>
                        </div>
                      </div>

                      {/* Max Tokens */}
                      <div className="space-y-3">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <Gauge className="h-3.5 w-3.5 text-muted-foreground" />
                            <Label className="text-sm font-medium">
                              {t('query.settings.maxTokens', 'Max Tokens')}
                            </Label>
                            <TooltipProvider>
                              <Tooltip>
                                <TooltipTrigger>
                                  <Info className="h-3.5 w-3.5 text-muted-foreground" />
                                </TooltipTrigger>
                                <TooltipContent side="top" className="max-w-[200px]">
                                  <p className="text-xs">{t('query.settings.maxTokensHint', 'Maximum length of the generated response')}</p>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          </div>
                          <Badge variant="secondary" className="font-mono text-xs">
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
                        <div className="flex justify-between text-xs text-muted-foreground">
                          <span>256 (Short)</span>
                          <span>4096 (Long)</span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </SheetContent>
            </Sheet>
          </div>
        </div>

        {/* Messages */}
        <ScrollArea ref={scrollRef} className="flex-1 p-4">
          <div className="max-w-3xl mx-auto">
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
          </div>
        </ScrollArea>

        {/* Input */}
        <div className="border-t p-4" role="form" aria-label={t('query.form', 'Query form')}>
          <form onSubmit={handleSubmit} className="max-w-3xl mx-auto">
            <div className="relative">
              <Textarea
                ref={inputRef}
                value={input}
                onChange={handleInputChange}
                placeholder={t('query.placeholder', 'Ask a question...')}
                className="min-h-[52px] max-h-[200px] resize-none pr-24 py-3"
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
              <div className="absolute right-2 bottom-2 flex items-center gap-1">
                {isLoading ? (
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={handleStop}
                    className="h-8"
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

      {/* History Sidebar */}
      <aside className="w-72 border-l bg-card overflow-auto" aria-label={t('query.history.title', 'Query history')}>
        <div className="p-4 space-y-4">
          {/* Favorites */}
          {favoriteQueries.length > 0 && (
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm flex items-center gap-2">
                  <Star className="h-4 w-4" />
                  Favorites
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-2">
                {favoriteQueries.slice(0, 5).map((item) => (
                  <div
                    key={item.id}
                    className="flex items-start gap-2 text-sm cursor-pointer hover:bg-muted p-2 rounded"
                    onClick={() => handleHistoryClick(item.query)}
                  >
                    <span className="flex-1 truncate">{item.query}</span>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6"
                      onClick={(e) => {
                        e.stopPropagation();
                        toggleFavorite(item.id);
                      }}
                    >
                      <Star className="h-3 w-3 fill-current" />
                    </Button>
                  </div>
                ))}
              </CardContent>
            </Card>
          )}

          {/* Recent */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm flex items-center gap-2">
                <History className="h-4 w-4" />
                Recent
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {recentQueries.length === 0 ? (
                <p className="text-sm text-muted-foreground">No recent queries</p>
              ) : (
                recentQueries.map((item) => (
                  <div
                    key={item.id}
                    className="flex items-start gap-2 text-sm cursor-pointer hover:bg-muted p-2 rounded"
                    onClick={() => handleHistoryClick(item.query)}
                  >
                    <span className="flex-1 truncate">{item.query}</span>
                    <div className="flex gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6"
                        onClick={(e) => {
                          e.stopPropagation();
                          toggleFavorite(item.id);
                        }}
                        aria-label={item.isFavorite ? t('query.history.unfavorite', 'Remove from favorites') : t('query.history.favorite', 'Add to favorites')}
                      >
                        <Star
                          className={`h-3 w-3 ${item.isFavorite ? 'fill-current' : ''}`}
                          aria-hidden="true"
                        />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6"
                        onClick={(e) => {
                          e.stopPropagation();
                          removeFromHistory(item.id);
                        }}
                        aria-label={t('query.history.remove', 'Remove from history')}
                      >
                        <Trash2 className="h-3 w-3" aria-hidden="true" />
                      </Button>
                    </div>
                  </div>
                ))
              )}
            </CardContent>
          </Card>
        </div>
      </aside>
    </div>
  );
}

export default QueryInterface;
