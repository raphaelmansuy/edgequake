/**
 * @module QueryInterface
 * @description Main query interface component for RAG knowledge graph queries.
 * Provides chat-based interaction with context-aware retrieval augmented generation.
 * 
 * @implements UC0201 - User submits a natural language query
 * @implements UC0202 - System retrieves relevant context from knowledge graph
 * @implements UC0203 - System generates augmented response with citations
 * @implements FEAT0007 - Natural Language Query Processing
 * @implements FEAT0101-0106 - Query mode selection (naive, local, global, hybrid, mix, bypass)
 * @implements FEAT0734 - Streaming responses with chain-of-thought display
 * 
 * @enforces BR0104 - Query response must include source citations
 * @enforces BR0105 - Streaming must show progressive thinking indicators
 * @enforces BR0401 - Conversation history persists across sessions
 * 
 * @see {@link docs/use_cases.md} UC0201-0203
 * @see {@link docs/features.md} FEAT0007, FEAT0101-0106
 */
'use client';

import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Textarea } from '@/components/ui/textarea';
import {
    useConversation,
    useConversations,
} from '@/hooks/use-conversations';
import { chatCompletion, chatCompletionStream } from '@/lib/api/chat';
import { ApiRequestError } from '@/lib/api/client';
import { deleteMessage } from '@/lib/api/conversations';
import { conversationKeys } from '@/lib/api/query-keys';
import { mapSourcesToContext } from '@/lib/utils/source-mapper';
import { generateUUID } from '@/lib/utils/uuid';
import { useActiveConversationId, useQueryUIStore } from '@/stores/use-query-ui-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { QueryContext, ServerMessage } from '@/types';
import { useQueryClient } from '@tanstack/react-query';
import {
    BookOpen,
    GitBranch,
    Lightbulb,
    Plus,
    Search,
    Send,
    Sparkles,
    StopCircle
} from 'lucide-react';
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { ChatMessage } from './chat-message';
import { ConversationHistoryPanelV2 } from './conversation-history-panel-v2';
import { MobileHistoryPanel } from './mobile-history-panel';
import { ProviderModelSelector } from './provider-model-selector';
import { LoadingMessage, NonStreamingLoadingIndicator } from './query-loading-indicators';
import { QueryModeSelector } from './query-mode-selector';
import { QuerySettingsSheet } from './query-settings-sheet';
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
  /** LLM provider used (lineage tracking). @implements SPEC-032 */
  llmProvider?: string;
  /** LLM model used (lineage tracking). @implements SPEC-032 */
  llmModel?: string;
}

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
  // Capture error/isError to handle stale conversation IDs gracefully
  const { 
    data: activeConversation, 
    isLoading: isLoadingConversation,
    error: conversationError,
    isError: isConversationError,
  } = useConversation(activeConversationId);
  
  // List conversations to auto-load most recent one if none is active
  const { data: conversationsData } = useConversations({
    sort: 'updated_at', // Get most recent first
  });
  
  // Handle stale conversation error (404) - auto-recover by clearing the stale ID
  // This happens when localStorage has a conversation ID that no longer exists on the server
  // (e.g., after backend restart with in-memory storage, or conversation was deleted)
  useEffect(() => {
    if (!isConversationError || !activeConversationId) return;
    
    // Check if this is a 404 "not found" error
    const is404Error = 
      (conversationError instanceof ApiRequestError && conversationError.status === 404) ||
      (conversationError instanceof Error && 
        conversationError.message.toLowerCase().includes('not found') && 
        conversationError.message.toLowerCase().includes('conversation'));
    
    if (is404Error) {
      // Clear the stale conversation ID
      store.setActiveConversation(null);
      
      // Show a friendly notification (not an error toast)
      toast(t('query.conversationExpired', 'Previous conversation not available'), {
        description: t('query.startingFreshSession', 'Starting a fresh session.'),
      });
    }
  }, [isConversationError, conversationError, activeConversationId, store, t]);
  
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
      store.setActiveConversation(mostRecentConversation.id);
    }
  }, [activeConversationId, conversationsData, store]);
  
  // Convert ServerMessage to local Message format
  const convertServerMessage = useCallback((msg: ServerMessage): Message => {
    // Convert ServerMessageContext to QueryContext format
    let context: QueryContext | undefined;
    if (msg.context) {
      // Filter sources by type
      const chunkSources = msg.context.sources?.filter(s => s.source_type === 'chunk' || !s.source_type) ?? [];
      
      // Helper to extract document UUID from chunk ID (format: "uuid-chunk-N" -> "uuid")
      const extractDocId = (chunkId: string): string => {
        const suffixIndex = chunkId.lastIndexOf('-chunk-');
        return suffixIndex > 0 ? chunkId.substring(0, suffixIndex) : chunkId;
      };
      
      context = {
        chunks: chunkSources.map(s => ({
          content: s.content,
          // Use document_id if provided, otherwise extract from chunk ID
          document_id: s.document_id ?? extractDocId(s.id),
          score: s.score,
          file_path: s.file_path,
        })),
        entities: msg.context.entities?.map(e => {
          // Handle both string[] and ServerContextEntity[] formats
          if (typeof e === 'string') {
            return {
              id: e,
              label: e,
              relevance: 1,
            };
          }
          return {
            id: e.name,
            label: e.name,
            relevance: e.score,
            source_document_id: e.source_document_id,
            source_file_path: e.source_file_path,
            source_chunk_ids: e.source_chunk_ids,
          };
        }) ?? [],
        relationships: msg.context.relationships?.map(r => {
          // Handle both string[] and ServerContextRelationship[] formats
          if (typeof r === 'string') {
            return {
              source: r,
              target: r,
              type: 'related',
              relevance: 1,
            };
          }
          return {
            source: r.source,
            target: r.target,
            type: r.relation_type,
            relevance: r.score,
            source_document_id: r.source_document_id,
            source_file_path: r.source_file_path,
          };
        }) ?? [],
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
      // SPEC-032: LLM provider/model lineage tracking
      llmProvider: msg.llm_provider ?? undefined,
      llmModel: msg.llm_model ?? undefined,
    };
  }, []);

  // Combine real messages with pending message (only when it has content)
  const messages = useMemo(() => {
    const serverMessages = (activeConversation?.messages ?? []).map(convertServerMessage);
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
    const messageId = generateUUID();
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
      // SPEC-032: Track LLM provider/model for lineage display
      let llmProvider: string | undefined;
      let llmModel: string | undefined;

      // Use the unified chat API - server handles message persistence
      // SPEC-032: Pass selected provider and model for query
      for await (const chunk of chatCompletionStream({
        conversation_id: conversationId || undefined,
        message: queryText,
        mode: querySettings.mode,
        max_tokens: querySettings.maxTokens,
        temperature: querySettings.temperature,
        top_k: querySettings.topK,
        stream: true,
        provider: querySettings.provider,
        model: querySettings.model,
      })) {
        if (abortControllerRef.current?.signal.aborted) {
          break;
        }

        switch (chunk.type) {
          case 'conversation':
            // Server created/confirmed conversation and saved user message
            newConversationId = chunk.conversation_id;
            if (!conversationId && newConversationId) {
              // New conversation was created - update UI
              store.setActiveConversation(newConversationId);
            }
            break;

          case 'context':
            // Map sources from API format to QueryContext for UI display
            if ('sources' in chunk && chunk.sources) {
              context = mapSourcesToContext(chunk.sources);
            }
            break;

          case 'token':
            fullContent += chunk.content;

            // Check if we transitioned from thinking to generating
            const parsed = parseCOTContent(fullContent);
            if (parsed.response && !thinkingTimeMs && thinkingStartRef.current) {
              thinkingTimeMs = Date.now() - thinkingStartRef.current;
              setStreamingState('generating');
            }

            // Update pending message with content and context
            setPendingMessage({
              ...assistantMessage,
              content: fullContent,
              thinkingTimeMs,
              context,  // Include context for SourceCitations display
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
            // SPEC-032: Capture LLM provider/model for lineage tracking
            llmProvider = chunk.llm_provider;
            llmModel = chunk.llm_model;
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
      }

      setStreamingState('complete');
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        setPendingMessage(null);
        setStreamingState('idle');
        return;
      }

      // Handle stale conversation ID (404 - Conversation not found)
      // This occurs when backend restarts (in-memory storage) or conversation was deleted
      const isConversationNotFound = 
        (error instanceof ApiRequestError && error.status === 404) ||
        (error instanceof Error && error.message.includes('not found') && error.message.toLowerCase().includes('conversation'));
      
      if (isConversationNotFound && conversationId) {
        // Clear the stale conversation and retry with a new one
        store.setActiveConversation(null);
        setPendingMessage(null);
        setStreamingState('idle');
        
        toast.warning(t('query.conversationExpired', 'Conversation expired'), {
          description: t('query.startingNewConversation', 'Starting a new conversation. Please submit your query again.'),
        });
        
        // Set the input back so user can easily retry
        // Note: We don't auto-retry to avoid potential loops
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
      // SPEC-032: Pass selected provider for query
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
          provider: querySettings.provider,
          model: querySettings.model,
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
        // Handle stale conversation ID (404 - Conversation not found)
        const isConversationNotFound = 
          (error instanceof ApiRequestError && error.status === 404) ||
          (error instanceof Error && error.message.includes('not found') && error.message.toLowerCase().includes('conversation'));
        
        if (isConversationNotFound && conversationId) {
          store.setActiveConversation(null);
          toast.warning(t('query.conversationExpired', 'Conversation expired'), {
            description: t('query.startingNewConversation', 'Starting a new conversation. Please submit your query again.'),
          });
          setStreamingState('idle');
          return;
        }
        
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

            {/* Provider & Model Selector (SPEC-032) */}
            <ProviderModelSelector
              value={querySettings.provider && querySettings.model 
                ? `${querySettings.provider}/${querySettings.model}` 
                : ''}
              onChange={(fullModelId) => {
                // Parse "provider/model" format from selector
                if (!fullModelId) {
                  // Server default - clear both
                  setQuerySettings({ provider: undefined, model: undefined });
                } else {
                  const parts = fullModelId.split('/');
                  const provider = parts[0];
                  const model = parts.slice(1).join('/'); // Handle model names with slashes
                  setQuerySettings({ provider, model });
                }
              }}
              disabled={isLoading}
            />

            {/* Mode Selector */}
            <QueryModeSelector
              value={querySettings.mode}
              onChange={(mode) => setQuerySettings({ mode })}
              disabled={isLoading}
            />

            {/* Settings */}
            <QuerySettingsSheet
              settings={{
                stream: querySettings.stream,
                topK: querySettings.topK,
                temperature: querySettings.temperature,
                maxTokens: querySettings.maxTokens,
              }}
              onSettingsChange={(updates) => setQuerySettings(updates)}
              disabled={isLoading}
            />
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
