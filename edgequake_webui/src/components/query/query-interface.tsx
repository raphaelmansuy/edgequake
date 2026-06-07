/**
 * @module QueryInterface
 * @description Main query interface component for RAG knowledge graph queries.
 *
 * @implements UC0201 - User submits a natural language query
 * @implements UC0202 - System retrieves relevant context from knowledge graph
 * @implements UC0203 - System generates augmented response with citations
 * @implements FEAT0007 - Natural Language Query Processing
 * @implements FEAT0101-0106 - Query mode selection (naive, local, global, hybrid, mix, bypass)
 * @implements FEAT0734 - Streaming responses with chain-of-thought display
 */
"use client";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Textarea } from "@/components/ui/textarea";
import { useQueryInterface } from "@/hooks/use-query-interface";
import { ImagePlus, Plus, Send, StopCircle, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ChatMessage } from "./chat-message";
import { ConversationHistoryPanelV2 } from "./conversation-history-panel-v2";
import { MobileHistoryPanel } from "./mobile-history-panel";
import { ProviderModelSelector } from "./provider-model-selector";
import { QueryDocumentFilter } from "./query-document-filter";
import { QueryEmptyState } from "./query-empty-state";
import { LoadingMessage, NonStreamingLoadingIndicator } from "./query-loading-indicators";
import { QueryModeSelector } from "./query-mode-selector";
import { QuerySettingsSheet } from "./query-settings-sheet";

export function QueryInterface() {
  const { t } = useTranslation();
  const {
    input,
    streamingState,
    pendingMessage,
    messages,
    isLoading,
    querySettings,
    setQuerySettings,
    scrollRef,
    scrollAnchorRef,
    inputRef,
    attachedImages,
    imageInputRef,
    maxImages,
    handleInputChange,
    handleSubmit,
    handleStop,
    handleRegenerate,
    handleSuggestionClick,
    handleNewConversation,
    handleImageInputChange,
    removeImage,
    handlePaste,
    handleDrop,
    handleDragOver,
  } = useQueryInterface();

  return (
    <div className="flex h-full min-h-0">
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        <header
          className="flex items-center justify-between border-b px-3 sm:px-5 py-3 shrink-0 bg-background/80 backdrop-blur-sm gap-2"
          role="banner"
        >
          <div className="flex items-center gap-2 sm:gap-3 min-w-0">
            <MobileHistoryPanel />
            <h1 className="text-base sm:text-lg font-semibold tracking-tight truncate">
              {t("query.title", "Query")}
            </h1>
            <span className="text-xs text-muted-foreground hidden md:inline">
              {t("query.subtitle", "Ask questions about your knowledge graph")}
            </span>
          </div>
          <div className="flex items-center gap-1.5 sm:gap-3 shrink-0">
            <Button
              variant="outline"
              size="sm"
              onClick={handleNewConversation}
              disabled={isLoading}
              className="gap-1"
            >
              <Plus className="h-4 w-4" />
              {t("query.newConversation", "New")}
            </Button>

            <ProviderModelSelector
              value={
                querySettings.provider && querySettings.model
                  ? `${querySettings.provider}/${querySettings.model}`
                  : ""
              }
              onChange={(fullModelId) => {
                if (!fullModelId) {
                  setQuerySettings({ provider: undefined, model: undefined });
                } else {
                  const parts = fullModelId.split("/");
                  const provider = parts[0];
                  const model = parts.slice(1).join("/");
                  setQuerySettings({ provider, model });
                }
              }}
              disabled={isLoading}
            />

            <QueryModeSelector
              value={querySettings.mode}
              onChange={(mode) => setQuerySettings({ mode })}
              disabled={isLoading}
            />

            <QueryDocumentFilter
              value={querySettings.documentFilter}
              onChange={(documentFilter) => setQuerySettings({ documentFilter })}
              disabled={isLoading}
            />

            <QuerySettingsSheet
              settings={{
                stream: querySettings.stream,
                topK: querySettings.topK,
                temperature: querySettings.temperature,
                maxTokens: querySettings.maxTokens,
                systemPrompt: querySettings.systemPrompt,
              }}
              onSettingsChange={(updates) => setQuerySettings(updates)}
              disabled={isLoading}
            />
          </div>
        </header>

        <div className="flex-1 min-h-0 overflow-hidden">
          <ScrollArea ref={scrollRef} className="h-full">
            <div
              className="max-w-4xl lg:max-w-5xl mx-auto px-4 sm:px-6 py-6"
              role="log"
              aria-live="polite"
              aria-label={t("query.messageList", "Conversation messages")}
            >
              {messages.length === 0 && !isLoading ? (
                <QueryEmptyState onSuggestionClick={handleSuggestionClick} />
              ) : (
                <>
                  {messages.map((message, index) => (
                    <ChatMessage
                      key={message.id}
                      message={message}
                      onRegenerate={
                        message.role === "assistant" &&
                        index === messages.length - 1
                          ? handleRegenerate
                          : undefined
                      }
                      isLast={index === messages.length - 1}
                    />
                  ))}
                  {isLoading &&
                    streamingState === "thinking" &&
                    (!pendingMessage || !pendingMessage.content) && (
                      <LoadingMessage />
                    )}
                  {isLoading &&
                    streamingState === "generating" &&
                    !pendingMessage && <NonStreamingLoadingIndicator />}
                </>
              )}
              <div ref={scrollAnchorRef} className="h-32" />
            </div>
          </ScrollArea>
        </div>

        <div
          className="border-t px-4 sm:px-6 py-4 bg-background shrink-0"
          role="form"
          aria-label={t("query.form", "Query form")}
        >
          <form onSubmit={handleSubmit} className="max-w-4xl lg:max-w-5xl mx-auto">
            <input
              ref={imageInputRef}
              type="file"
              accept="image/jpeg,image/png,image/gif,image/webp"
              multiple
              className="sr-only"
              aria-label={t("query.attachImages", "Attach images")}
              onChange={handleImageInputChange}
            />
            {attachedImages.length > 0 && (
              <div
                className="flex flex-wrap gap-2 mb-2"
                role="list"
                aria-label={t("query.attachedImages", "Attached images")}
              >
                {attachedImages.map((img, idx) => (
                  <div
                    key={idx}
                    role="listitem"
                    className="relative group w-16 h-16 rounded border overflow-hidden flex-shrink-0"
                  >
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img
                      src={img.preview}
                      alt={`Attachment ${idx + 1}`}
                      className="w-full h-full object-cover"
                    />
                    <button
                      type="button"
                      onClick={() => removeImage(idx)}
                      className="absolute top-0.5 right-0.5 bg-black/60 rounded-full p-0.5 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity"
                      aria-label={t("query.removeImage", `Remove image ${idx + 1}`)}
                    >
                      <X className="h-3 w-3 text-white" aria-hidden="true" />
                    </button>
                  </div>
                ))}
              </div>
            )}
            <div
              className="relative"
              onDrop={handleDrop}
              onDragOver={handleDragOver}
            >
              <Textarea
                ref={inputRef}
                value={input}
                onChange={handleInputChange}
                onPaste={handlePaste}
                placeholder={t("query.placeholder", "Ask a question...")}
                className="min-h-[56px] max-h-[200px] resize-none pr-24 py-4 text-base query-input focus-visible:ring-2 focus-visible:ring-primary/30 focus-visible:border-primary transition-all duration-200"
                rows={1}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && !event.shiftKey) {
                    event.preventDefault();
                    void handleSubmit();
                  }
                }}
                disabled={isLoading}
                aria-label={t("query.placeholder", "Ask a question")}
                aria-describedby="query-hint"
              />
              <span id="query-hint" className="sr-only">
                Press Enter to send, Shift+Enter for new line
              </span>
              <div className="absolute right-3 bottom-3 flex items-center gap-2">
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  onClick={() => imageInputRef.current?.click()}
                  disabled={isLoading || attachedImages.length >= maxImages}
                  className="h-8 w-8 p-0"
                  aria-label={t("query.attachImages", "Attach images")}
                  title={t("query.attachImages", "Attach images")}
                >
                  <ImagePlus className="h-4 w-4" aria-hidden="true" />
                </Button>
                {isLoading ? (
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={handleStop}
                    className="h-9"
                    aria-label={t("query.stop", "Stop generating")}
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
                    aria-label={t("query.submit", "Send message")}
                  >
                    <Send className="h-4 w-4" aria-hidden="true" />
                  </Button>
                )}
              </div>
            </div>
            <p
              className="text-xs text-muted-foreground mt-2 text-center"
              aria-hidden="true"
            >
              {t("query.hint", "Press Enter to send, Shift+Enter for new line")}
            </p>
          </form>
        </div>
      </div>

      <ConversationHistoryPanelV2 />
    </div>
  );
}

export default QueryInterface;
