'use client';

import { useState, useRef, useEffect } from 'react';
import { useMutation } from '@tanstack/react-query';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { toast } from 'sonner';
import {
  Send,
  Loader2,
  Settings2,
  History,
  Star,
  Trash2,
  Target,
  Globe,
  Combine,
  Sparkles,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import { query as queryApi, queryStream } from '@/lib/api/edgequake';
import { useQueryStore, useRecentQueries, useFavoriteQueries } from '@/stores/use-query-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import { QueryModeSelector } from './query-mode-selector';
import type { QueryMode, QueryResponse, QueryStreamChunk } from '@/types';

const modeConfig = {
  local: {
    icon: Target,
    label: 'Local',
    description: 'Search within entity neighborhood',
  },
  global: {
    icon: Globe,
    label: 'Global',
    description: 'Search entire knowledge graph',
  },
  hybrid: {
    icon: Combine,
    label: 'Hybrid',
    description: 'Combine local and global search',
  },
  naive: {
    icon: Sparkles,
    label: 'Naive',
    description: 'Simple keyword search',
  },
} as const;

interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  mode?: QueryMode;
  tokensUsed?: number;
  durationMs?: number;
}

export function QueryInterface() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  const scrollRef = useRef<HTMLDivElement>(null);

  const { querySettings, setQuerySettings } = useSettingsStore();
  const { addToHistory, toggleFavorite, removeFromHistory } = useQueryStore();
  const recentQueries = useRecentQueries(10);
  const favoriteQueries = useFavoriteQueries();

  // Scroll to bottom when messages change
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, streamingContent]);

  const handleStreamQuery = async (queryText: string) => {
    setIsStreaming(true);
    setStreamingContent('');

    try {
      let fullContent = '';
      let tokensUsed = 0;
      let durationMs = 0;

      for await (const chunk of queryStream({
        query: queryText,
        mode: querySettings.mode,
        top_k: querySettings.topK,
        max_tokens: querySettings.maxTokens,
        temperature: querySettings.temperature,
        stream: true,
      })) {
        if (chunk.type === 'token' && chunk.content) {
          fullContent += chunk.content;
          setStreamingContent(fullContent);
        } else if (chunk.type === 'done') {
          tokensUsed = chunk.tokens_used || 0;
          durationMs = chunk.duration_ms || 0;
        } else if (chunk.type === 'error') {
          throw new Error(chunk.error || 'Streaming failed');
        }
      }

      // Add to messages
      setMessages((prev) => [
        ...prev,
        {
          id: crypto.randomUUID(),
          role: 'assistant',
          content: fullContent,
          mode: querySettings.mode,
          tokensUsed,
          durationMs,
        },
      ]);

      // Add to history
      addToHistory({
        query: queryText,
        mode: querySettings.mode,
        response: fullContent,
        isFavorite: false,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Query failed';
      toast.error(message);
    } finally {
      setIsStreaming(false);
      setStreamingContent('');
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
      setMessages((prev) => [
        ...prev,
        {
          id: crypto.randomUUID(),
          role: 'assistant',
          content: data.answer,
          mode: data.mode,
          tokensUsed: data.tokens_used,
          durationMs: data.duration_ms,
        },
      ]);

      addToHistory({
        query: queryText,
        mode: data.mode,
        response: data.answer,
        isFavorite: false,
      });
    },
    onError: (error) => {
      toast.error(`Query failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    },
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim() || isStreaming || queryMutation.isPending) return;

    const queryText = input.trim();
    setInput('');

    // Add user message
    setMessages((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        role: 'user',
        content: queryText,
      },
    ]);

    // Use streaming or regular query
    if (querySettings.stream) {
      await handleStreamQuery(queryText);
    } else {
      queryMutation.mutate(queryText);
    }
  };

  const handleHistoryClick = (query: string) => {
    setInput(query);
  };

  const isLoading = isStreaming || queryMutation.isPending;

  return (
    <div className="flex h-full">
      {/* Main Query Area */}
      <div className="flex-1 flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between border-b px-4 py-3">
          <div>
            <h1 className="text-lg font-semibold">Query</h1>
            <p className="text-sm text-muted-foreground">
              Ask questions about your knowledge graph
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
              <SheetContent>
                <SheetHeader>
                  <SheetTitle>Query Settings</SheetTitle>
                </SheetHeader>
                <div className="space-y-6 mt-6">
                  {/* Stream Toggle */}
                  <div className="flex items-center justify-between">
                    <div>
                      <label className="text-sm font-medium">Streaming</label>
                      <p className="text-xs text-muted-foreground">
                        Show response as it generates
                      </p>
                    </div>
                    <Switch
                      checked={querySettings.stream}
                      onCheckedChange={(stream) => setQuerySettings({ stream })}
                    />
                  </div>

                  {/* Top K */}
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <label className="text-sm font-medium">Top K Results</label>
                      <span className="text-sm text-muted-foreground">{querySettings.topK}</span>
                    </div>
                    <Slider
                      value={[querySettings.topK]}
                      onValueChange={([topK]) => setQuerySettings({ topK })}
                      min={1}
                      max={50}
                      step={1}
                    />
                  </div>

                  {/* Temperature */}
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <label className="text-sm font-medium">Temperature</label>
                      <span className="text-sm text-muted-foreground">
                        {querySettings.temperature}
                      </span>
                    </div>
                    <Slider
                      value={[querySettings.temperature]}
                      onValueChange={([temperature]) => setQuerySettings({ temperature })}
                      min={0}
                      max={2}
                      step={0.1}
                    />
                  </div>

                  {/* Max Tokens */}
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <label className="text-sm font-medium">Max Tokens</label>
                      <span className="text-sm text-muted-foreground">{querySettings.maxTokens}</span>
                    </div>
                    <Slider
                      value={[querySettings.maxTokens]}
                      onValueChange={([maxTokens]) => setQuerySettings({ maxTokens })}
                      min={256}
                      max={4096}
                      step={256}
                    />
                  </div>
                </div>
              </SheetContent>
            </Sheet>
          </div>
        </div>

        {/* Messages */}
        <ScrollArea ref={scrollRef} className="flex-1 p-4">
          <div className="max-w-3xl mx-auto space-y-4">
            {messages.length === 0 && !streamingContent && (
              <div className="text-center py-12">
                <Sparkles className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
                <h2 className="text-lg font-medium">Start a conversation</h2>
                <p className="text-sm text-muted-foreground mt-1">
                  Ask questions about your knowledge graph
                </p>
              </div>
            )}

            {messages.map((message) => (
              <div
                key={message.id}
                className={`flex ${message.role === 'user' ? 'justify-end' : 'justify-start'}`}
              >
                <Card
                  className={`max-w-[80%] ${
                    message.role === 'user' ? 'bg-primary text-primary-foreground' : ''
                  }`}
                >
                  <CardContent className="p-3">
                    {message.role === 'assistant' ? (
                      <div className="prose prose-sm dark:prose-invert max-w-none">
                        <ReactMarkdown remarkPlugins={[remarkGfm]}>
                          {message.content}
                        </ReactMarkdown>
                        {message.tokensUsed && (
                          <div className="flex gap-2 mt-2 text-xs text-muted-foreground">
                            <Badge variant="secondary">{message.mode}</Badge>
                            <span>{message.tokensUsed} tokens</span>
                            <span>{message.durationMs}ms</span>
                          </div>
                        )}
                      </div>
                    ) : (
                      <p>{message.content}</p>
                    )}
                  </CardContent>
                </Card>
              </div>
            ))}

            {/* Streaming response */}
            {streamingContent && (
              <div className="flex justify-start">
                <Card className="max-w-[80%]">
                  <CardContent className="p-3">
                    <div className="prose prose-sm dark:prose-invert max-w-none">
                      <ReactMarkdown remarkPlugins={[remarkGfm]}>
                        {streamingContent}
                      </ReactMarkdown>
                      <span className="inline-block w-2 h-4 bg-foreground animate-pulse ml-1" />
                    </div>
                  </CardContent>
                </Card>
              </div>
            )}
          </div>
        </ScrollArea>

        {/* Input */}
        <div className="border-t p-4">
          <form onSubmit={handleSubmit} className="max-w-3xl mx-auto">
            <div className="flex gap-2">
              <Textarea
                value={input}
                onChange={(e) => setInput(e.target.value)}
                placeholder="Ask a question..."
                className="min-h-[60px] resize-none"
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    handleSubmit(e);
                  }
                }}
              />
              <Button type="submit" disabled={!input.trim() || isLoading}>
                {isLoading ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Send className="h-4 w-4" />
                )}
              </Button>
            </div>
          </form>
        </div>
      </div>

      {/* History Sidebar */}
      <div className="w-72 border-l bg-card overflow-auto">
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
                      >
                        <Star
                          className={`h-3 w-3 ${item.isFavorite ? 'fill-current' : ''}`}
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
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  </div>
                ))
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}

export default QueryInterface;
