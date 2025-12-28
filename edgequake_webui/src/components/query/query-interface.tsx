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
import {
    useConversation,
    useConversations,
} from '@/hooks/use-conversations';
import { chatCompletion, chatCompletionStream } from '@/lib/api/chat';
import { deleteMessage } from '@/lib/api/conversations';
import { conversationKeys } from '@/lib/api/query-keys';
import { useActiveConversationId, useQueryUIStore } from '@/stores/use-query-ui-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { QueryContext, ServerMessage } from '@/types';
import { useQueryClient } from '@tanstack/react-query';
import {
    BookOpen,
    Brain,
    Gauge,
    GitBranch,
    Info,
    Lightbulb,
    Plus,
    Search,
    Send,
    Settings2,
    Sliders,
    Sparkles,
    StopCircle,
    Thermometer,
    Zap
} from 'lucide-react';
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { ChatMessage } from './chat-message';
import { ConversationHistoryPanelV2 } from './conversation-history-panel-v2';
import { MobileHistoryPanel } from './mobile-history-panel';
import { QueryModeSelector } from './query-mode-selector';
import { parseCOTContent } from './thinking-display';

// Streaming state for better UX
type StreamingState = 'idle' | 'thinking' | 'generating' | 'complete' | 'error';

// Query mode type
type QueryModeType = 'local' | 'global' | 'hybrid' | 'naive';

// Message type compatible with ChatMessageData
interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  mode?: QueryModeType;
  tokensUsed?: number;
  durationMs?: number;
  thinkingTimeMs?: number;
  context?: QueryContext;
  isError?: boolean;
  isStreaming?: boolean;
  timestamp?: number;
}

// ============================================================================
// Delightful Loading Indicator - Shows minimal, smooth placeholder while waiting
// ============================================================================

const LoadingMessage = memo(function LoadingMessage() {
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

// ============================================================================
// Non-Streaming Loading Indicator - Delightful multi-phase animation
// Shows a sophisticated loading experience with visual progression
// ============================================================================

const NonStreamingLoadingIndicator = memo(function NonStreamingLoadingIndicator() {
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
                
                {/* Progress bar - no animation to avoid visual artifacts */}
                <div className="mt-2 h-1 w-full bg-muted rounded-full overflow-hidden">
                  <div className="h-full w-full bg-gradient-to-r from-primary/40 via-primary to-primary/40 rounded-full" />
                </div>
              </div>
            </div>

            {/* Phase dots */}
            <div className="flex items-center justify-center gap-2 mt-3">
              {phases.map((_, i) => (
                <span
                  key={i}
                  className={`h-1.5 rounded-full transition-all duration-300 ${
                    i === phase 
                      ? 'w-4 bg-primary' 
                      : i < phase 
                        ? 'w-1.5 bg-primary/40' 
                        : 'w-1.5 bg-muted-foreground/20'
                  }`}
                />
              ))}
            </div>
          </div>
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
  const [pendingMessage, setPendingMessage] = useState<Message | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const scrollAnchorRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const thinkingStartRef = useRef<number | null>(null);
  const hasInitializedRef = useRef(false);

  const queryClient = useQueryClient();
  const { querySettings, setQuerySettings } = useSettingsStore();
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  
  // Use the new server-synced state
  const store = useQueryUIStore();
  const activeConversationId = useActiveConversationId();
  
  // Server state for active conversation
  const { data: activeConversation, isLoading: isLoadingConversation } = useConversation(activeConversationId);
  
  // List conversations to auto-load most recent one if none is active
  const { data: conversationsData } = useConversations({
    sort: 'updated_at', // Get most recent first
  });
  
  // Auto-load most recent conversation on mount if none is active
  // Only do this once on initial mount, not when user clicks "New"
  useEffect(() => {
    // Skip auto-loading if already initialized (e.g., user clicked "New" button)
    if (hasInitializedRef.current) {
      return;
    }
    
    // Mark as initialized to prevent future auto-loads
    hasInitializedRef.current = true;
    
    // Only auto-load if we have conversations and no active conversation
    const firstPage = conversationsData?.pages?.[0];
    if (!activeConversationId && firstPage?.items && firstPage.items.length > 0) {
      const mostRecentConversation = firstPage.items[0];
      console.log('Auto-loading most recent conversation:', mostRecentConversation.id);
      store.setActiveConversation(mostRecentConversation.id);
    }
  }, [activeConversationId, conversationsData, store]);
  
  // Convert ServerMessage to local Message format
  const convertServerMessage = useCallback((msg: ServerMessage): Message => {
    // Convert ServerMessageContext to QueryContext format
    let context: QueryContext | undefined;
    if (msg.context) {
      context = {
        chunks: msg.context.sources?.map(s => ({
          content: s.content,
          document_id: s.id,
          score: s.score,
        })) ?? [],
        entities: msg.context.entities?.map(e => ({
          id: e,
          label: e,
          relevance: 1,
        })) ?? [],
        relationships: msg.context.relationships?.map(r => ({
          source: r,
          target: r,
          type: 'related',
          relevance: 1,
        })) ?? [],
      };
    }
    
    return {
      id: msg.id,
      role: msg.role as 'user' | 'assistant',
      content: msg.content,
      mode: (msg.mode as QueryModeType) ?? undefined,
      tokensUsed: msg.tokens_used ?? undefined,
      durationMs: msg.duration_ms ?? undefined,
      thinkingTimeMs: msg.thinking_time_ms ?? undefined,
      context,
      isError: msg.is_error,
      isStreaming: false,
      timestamp: new Date(msg.created_at).getTime(),
    };
  }, []);

  // Combine real messages with pending message (only when it has content)
  const messages = useMemo(() => {
    const serverMessages = (activeConversation?.messages ?? []).map(convertServerMessage);
    console.log('📨 Messages loaded:', {
      conversationId: activeConversationId,
      serverMessageCount: serverMessages.length,
      hasPending: !!pendingMessage,
      pendingHasContent: !!(pendingMessage?.content)
    });
    // Only include pendingMessage when it has actual content to avoid two bubbles
    // (LoadingMessage handles the empty "thinking" state)
    if (pendingMessage && pendingMessage.content) {
      return [...serverMessages, pendingMessage];
    }
    return serverMessages;
  }, [activeConversation?.messages, pendingMessage, convertServerMessage, activeConversationId]);

  // Handle tenant/workspace change - start fresh
  useEffect(() => {
    // Only handle context change if there's an active conversation
    if (activeConversationId && messages.length > 0) {
      // Clear active conversation to start fresh
      store.setActiveConversation(null);
      setPendingMessage(null);
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

  const handleStreamQuery = useCallback(async (queryText: string, conversationId: string | null) => {
    const messageId = crypto.randomUUID();
    setStreamingState('thinking');
    thinkingStartRef.current = Date.now();
    abortControllerRef.current = new AbortController();

    // Add placeholder pending message
    const assistantMessage: Message = {
      id: messageId,
      role: 'assistant',
      content: '',
      mode: querySettings.mode,
      isStreaming: true,
      timestamp: Date.now(),
    };
    setPendingMessage(assistantMessage);

    try {
      let fullContent = '';
      let tokensUsed = 0;
      let durationMs = 0;
      let context: QueryContext | undefined;
      let thinkingTimeMs: number | undefined;
      let newConversationId = conversationId;
      let assistantMessageId: string | undefined;

      // Use the unified chat API - server handles message persistence
      for await (const chunk of chatCompletionStream({
        conversation_id: conversationId || undefined,
        message: queryText,
        mode: querySettings.mode,
        max_tokens: querySettings.maxTokens,
        temperature: querySettings.temperature,
        top_k: querySettings.topK,
        stream: true,
      })) {
        if (abortControllerRef.current?.signal.aborted) {
          break;
        }

        switch (chunk.type) {
          case 'conversation':
            // Server created/confirmed conversation and saved user message
            newConversationId = chunk.conversation_id;
            console.log('✓ Conversation created/confirmed:', newConversationId);
            if (!conversationId && newConversationId) {
              // New conversation was created - update UI
              store.setActiveConversation(newConversationId);
              console.log('✓ Active conversation set:', newConversationId);
            }
            break;

          case 'context':
            // Sources retrieved - could display inline
            // context = ...; // Convert from ChatStreamEvent sources to QueryContext
            break;

          case 'token':
            fullContent += chunk.content;

            // Check if we transitioned from thinking to generating
            const parsed = parseCOTContent(fullContent);
            if (parsed.response && !thinkingTimeMs && thinkingStartRef.current) {
              thinkingTimeMs = Date.now() - thinkingStartRef.current;
              setStreamingState('generating');
            }

            // Update pending message
            setPendingMessage({
              ...assistantMessage,
              content: fullContent,
              thinkingTimeMs,
            });
            break;

          case 'thinking':
            // Thinking phase content - could display separately
            break;

          case 'done':
            // Server has saved the assistant message
            assistantMessageId = chunk.assistant_message_id;
            tokensUsed = chunk.tokens_used || 0;
            durationMs = chunk.duration_ms || 0;
            console.log('✓ Message saved on server:', assistantMessageId, {tokensUsed, durationMs});
            break;

          case 'error':
            throw new Error(chunk.message || 'Streaming failed');
        }
      }

      // Clear pending message
      setPendingMessage(null);
      
      // Server already saved both user and assistant messages!
      // Just refresh the conversation data from server
      if (newConversationId) {
        // Force refetch the conversation to get updated messages
        await queryClient.invalidateQueries({ 
          queryKey: conversationKeys.detail(newConversationId) 
        });
        await queryClient.invalidateQueries({ 
          queryKey: conversationKeys.lists() 
        });
        
        // Give React Query a moment to refetch
        await new Promise(resolve => setTimeout(resolve, 100));
        
        console.log('✓ Conversation data refreshed:', newConversationId);
      }

      setStreamingState('complete');
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        setPendingMessage(null);
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

      // Show error in pending message
      setPendingMessage({
        ...assistantMessage,
        content: errorMessage,
        isStreaming: false,
        isError: true,
      });

      setStreamingState('error');
    } finally {
      abortControllerRef.current = null;
      thinkingStartRef.current = null;
    }
  }, [querySettings, queryClient, store, t]);

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    
    // Guard against empty input or double-submission while loading
    const isStreamingOrLoading = streamingState === 'thinking' || streamingState === 'generating';
    if (!input.trim() || isStreamingOrLoading) return;

    const queryText = input.trim();
    setInput('');
    
    // Reset textarea height
    if (inputRef.current) {
      inputRef.current.style.height = 'auto';
    }

    // The unified chat API handles conversation creation and message persistence
    // We just pass the current conversation ID (or null for a new one)
    const conversationId = activeConversationId;

    // Use streaming or regular query
    // The chat API will create a conversation if conversationId is null
    if (querySettings.stream) {
      await handleStreamQuery(queryText, conversationId);
    } else {
      // Non-streaming: use the unified chat API
      // Server handles conversation creation and message persistence
      setStreamingState('generating');
      try {
        const response = await chatCompletion({
          conversation_id: conversationId || undefined,
          message: queryText,
          mode: querySettings.mode,
          max_tokens: querySettings.maxTokens,
          temperature: querySettings.temperature,
          top_k: querySettings.topK,
          stream: false,
        });

        // Update active conversation if a new one was created
        if (!conversationId && response.conversation_id) {
          store.setActiveConversation(response.conversation_id);
        }

        // Refresh conversation data from server
        await queryClient.invalidateQueries({
          queryKey: conversationKeys.detail(response.conversation_id),
        });
        await queryClient.invalidateQueries({
          queryKey: conversationKeys.all,
        });
        setStreamingState('complete');
      } catch (error) {
        toast.error(t('query.failed', 'Query failed'), {
          description: error instanceof Error ? error.message : t('common.unknownError', 'Unknown error'),
        });
        setStreamingState('error');
      }
    }
  };

  // Handle regenerate - delete old assistant AND user message, then generate fresh pair
  const handleRegenerate = useCallback(async () => {
    if (!activeConversationId || messages.length < 2) return;
    
    // Find the last user message and the last assistant message
    const lastUserMessage = [...messages].reverse().find((m) => m.role === 'user');
    const lastAssistantMessage = [...messages].reverse().find((m) => m.role === 'assistant');
    
    if (!lastUserMessage) return;

    // Save the query text before deleting
    const queryText = lastUserMessage.content;

    // Clear pending message immediately
    setPendingMessage(null);

    try {
      // Delete BOTH the old assistant AND user messages from server
      // This prevents duplicate user messages since handleStreamQuery will create a fresh pair
      const deletePromises = [];
      
      if (lastAssistantMessage && !lastAssistantMessage.isStreaming) {
        deletePromises.push(deleteMessage(lastAssistantMessage.id));
      }
      if (lastUserMessage) {
        deletePromises.push(deleteMessage(lastUserMessage.id));
      }
      
      await Promise.all(deletePromises);
      
      // Invalidate the conversation cache to remove the old messages from UI
      await queryClient.invalidateQueries({ 
        queryKey: conversationKeys.detail(activeConversationId) 
      });
    } catch (error) {
      console.error('Failed to delete old messages:', error);
      // Continue with regeneration even if delete fails
    }

    // Regenerate with the same user query - server will create fresh user+assistant pair
    handleStreamQuery(queryText, activeConversationId);
  }, [messages, activeConversationId, handleStreamQuery, queryClient]);

  // Handle suggestion click
  const handleSuggestionClick = useCallback((text: string) => {
    setInput(text);
    inputRef.current?.focus();
  }, []);

  // Handle new conversation
  const handleNewConversation = useCallback(() => {
    store.setActiveConversation(null);
    setPendingMessage(null);
    setInput('');
    setStreamingState('idle');
  }, [store]);

  const isLoading = streamingState === 'thinking' || streamingState === 'generating' || isLoadingConversation;

  return (
    <div className="flex h-full min-h-0">
      {/* Main Query Area */}
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {/* Header */}
        <header className="flex items-center justify-between border-b px-5 py-3 shrink-0 bg-background/80 backdrop-blur-sm">
          <div className="flex items-center gap-3">
            {/* Mobile History Panel Toggle */}
            <MobileHistoryPanel />
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
              {messages.length === 0 && !isLoading ? (
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
                  {/* Show loading message only during thinking phase AND when pending has no content yet */}
                  {/* Once content arrives in pendingMessage, the ChatMessage component will render it */}
                  {isLoading && streamingState === 'thinking' && (!pendingMessage || !pendingMessage.content) && <LoadingMessage />}
                  {/* Show loading during non-streaming mode (when generating without pending content) */}
                  {isLoading && streamingState === 'generating' && !pendingMessage && <NonStreamingLoadingIndicator />}
                </>
              )}
              {/* Scroll anchor for auto-scroll - height matches input area to ensure visibility */}
              <div ref={scrollAnchorRef} className="h-32" />
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

      {/* Conversation History Panel - Server-synced V2 component */}
      <ConversationHistoryPanelV2 />
    </div>
  );
}

export default QueryInterface;
