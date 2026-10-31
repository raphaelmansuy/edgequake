/**
 * @module QueryInterface
 * @description Main query interface — composer, stream, history (SPEC-155 W7Q).
 */
"use client";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  TooltipProvider,
} from "@/components/ui/tooltip";
import { useQueryInterface } from "@/hooks/use-query-interface";
import { useMediaQuery } from "@/hooks/use-media-query";
import { useQueryScope } from "@/hooks/use-query-scope";
import { useCompanionLayout } from "@/hooks/use-companion-layout";
import { useWorkspaceCapabilities } from "@/hooks/use-workspace-capabilities";
import {
  entityHandoffFromNode,
  useQueryHandoff,
} from "@/hooks/use-query-handoff";
import type { GraphNode } from "@/types/graph";
import { isCompanionEnabled } from "@/lib/query/companion-flag";
import { useCompanionPaneStore } from "@/stores/use-companion-pane-store";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import { ArrowDown, Columns2, PanelRight, Plus } from "lucide-react";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChatMessage } from "./message";
import { Composer } from "./composer";
import { ModelChip } from "./composer/model-chip";
import { useQueryChoice } from "@/hooks/use-query-choice";
import { SlashMenu } from "./composer/slash-menu";
import { useQueryComposerShortcuts } from "./composer/use-query-composer-shortcuts";
import { ConversationHistoryPanelV2 } from "./conversation-history-panel-v2";
import { MobileHistoryPanel } from "./mobile-history-panel";
import { getQueryEmptyCopy } from "@/lib/query/query-empty-copy";
import { QueryEmptyState } from "./query-empty-state";
import { CompanionShell } from "./companion/companion-shell";
import { QuerySettingsSheet } from "./query-settings-sheet";

export function QueryInterface() {
  const { t } = useTranslation();
  const choice = useQueryChoice();
  const isXl = useMediaQuery("(min-width: 1280px)");
  const historyOpen = useQueryUIStore((s) => s.historyPanelOpen);
  const toggleHistory = useQueryUIStore((s) => s.toggleHistoryPanel);

  const {
    input,
    streamingState,
    stage,
    stageDetail,
    pendingMessage,
    messages,
    isLoading,
    isStreaming,
    querySettings,
    setQuerySettings,
    scrollRef,
    scrollAnchorRef,
    showJumpPill,
    jumpToLatest,
    inputRef,
    attachedImages,
    searchImage,
    imageInputRef,
    maxImages,
    handleInputChange,
    handleSubmit,
    handleStop,
    handleRegenerate,
    handleRetry,
    handleSuggestionClick,
    handleEditMessage,
    handleNewConversation,
    handleImageInputChange,
    removeImage,
    handlePaste,
    handleDrop,
    handleDragOver,
    queuedMessage,
    queueMessage,
    clearQueue,
  } = useQueryInterface();

  const [scopePickerOpen, setScopePickerOpen] = useState(false);
  const [slashMenuOpen, setSlashMenuOpen] = useState(false);
  const scope = useQueryScope();
  const { rootRef, layout, history, companionOpen } = useCompanionLayout(isXl);
  const historyDock = history !== "overlay";
  const companionEnabled = isCompanionEnabled();
  // SPEC-180: the split opens a graph; a workspace without one has nothing to split to.
  const showsGraph = useWorkspaceCapabilities().shows_graph_ui;
  const emptyCopy = getQueryEmptyCopy(querySettings.mode, showsGraph);
  const closeCompanion = useCompanionPaneStore((s) => s.close);
  const openSource = useCompanionPaneStore((s) => s.openSource);
  const openWorkspaceGraph = useCompanionPaneStore((s) => s.openWorkspaceGraph);

  const toggleSplit = useCallback(() => {
    if (companionOpen) {
      closeCompanion();
      return;
    }
    const docId = scope.ids[0];
    if (docId) {
      openSource({
        documentId: docId,
        page: 1,
        title: scope.titles[docId],
      });
      return;
    }
    openWorkspaceGraph();
  }, [
    companionOpen,
    closeCompanion,
    openSource,
    openWorkspaceGraph,
    scope.ids,
    scope.titles,
  ]);

  const focusComposer = useCallback(() => {
    inputRef.current?.focus();
  }, [inputRef]);

  const openSlashMenu = useCallback(() => setSlashMenuOpen(true), []);

  // SPEC-157 W4: quote a verified passage into the composer, ready to ask.
  const quotePassage = useCallback(
    (text: string) => {
      const quoted = text
        .split("\n")
        .map((line) => `> ${line}`)
        .join("\n");
      const lead = input && !input.endsWith("\n") ? "\n" : "";
      handleInputChange(`${input}${lead}${quoted}\n\n`);
      requestAnimationFrame(() => {
        const el = inputRef.current;
        if (!el) return;
        el.focus();
        el.setSelectionRange(el.value.length, el.value.length);
      });
    },
    [handleInputChange, input, inputRef],
  );

  // SPEC-159: Ask always starts a new conversation (same handoff as graph/doc).
  const { ask } = useQueryHandoff();
  const askEntity = useCallback(
    (node: GraphNode) => {
      ask(entityHandoffFromNode(node));
    },
    [ask],
  );

  // Global `@`: focus the composer and seed a mention token (menu opens from the text).
  const startMention = useCallback(() => {
    const needsSpace = input.length > 0 && !/\s$/.test(input);
    handleInputChange(`${input}${needsSpace ? " " : ""}@`);
    requestAnimationFrame(() => {
      const el = inputRef.current;
      if (!el) return;
      el.focus();
      el.setSelectionRange(el.value.length, el.value.length);
    });
  }, [handleInputChange, input, inputRef]);

  useQueryComposerShortcuts({
    inputRef,
    input,
    onStartMention: startMention,
    onOpenSlashMenu: openSlashMenu,
    onFocusComposer: focusComposer,
  });

  const statusAnnouncement = (() => {
    // Prefer stream-session stage (state machine) over coarse streamingState
    if (stage === "retrieving") {
      return t("query.stage.retrieving", "Searching knowledge…");
    }
    if (stage === "reading") {
      return t("query.stage.reading", "Reading sources…");
    }
    if (stage === "thinking") {
      return t("query.stage.thinking", "Thinking…");
    }
    if (stage === "generating" || streamingState === "generating") {
      return t("query.stage.generating", "Writing answer…");
    }
    if (stage === "complete" || streamingState === "complete") {
      return t("query.stage.complete", "Answer ready");
    }
    if (stage === "error" || streamingState === "error") {
      return t("query.stage.error", "Failed");
    }
    if (stage === "stopped") {
      return t("query.stage.stopped", "Stopped");
    }
    if (streamingState === "thinking") {
      return t("query.stage.retrieving", "Searching knowledge…");
    }
    return "";
  })();

  return (
    <TooltipProvider delayDuration={200}>
      <div ref={rootRef} className="flex h-full min-h-0">
        <div className="flex-1 min-w-0 flex flex-col min-h-0 overflow-hidden">
          <header
            className="eq-toolbar eq-gutter justify-between border-b py-2 shrink-0 bg-background/80 backdrop-blur-sm"
            role="banner"
          >
            <div className="flex items-center gap-2 sm:gap-3 min-w-0">
              {!historyDock ? <MobileHistoryPanel /> : null}
              <h1 className="shrink-0 text-[1.375rem] font-semibold leading-snug tracking-[-0.012em]">
                {t("query.title", "Query")}
              </h1>
              <span className="hidden min-w-0 truncate whitespace-nowrap text-xs text-muted-foreground xl:inline">
                {t(emptyCopy.keys.subtitle, emptyCopy.subtitle)}
              </span>
            </div>
            <div className="eq-toolbar min-w-0 shrink-0">
              <Button
                variant="outline"
                size="sm"
                onClick={handleNewConversation}
                className="gap-1"
                data-testid="query-new"
              >
                <Plus className="h-4 w-4" />
                <span className="hidden sm:inline">
                  {t("query.newConversation", "New")}
                </span>
              </Button>

              {companionEnabled && showsGraph ? (
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={toggleSplit}
                  aria-label={
                    companionOpen
                      ? t("query.companion.close", "Close pane")
                      : t(
                          "query.companion.splitGraph",
                          "Show graph beside chat",
                        )
                  }
                  aria-pressed={companionOpen}
                  title={t("query.companion.split", "Split")}
                  data-testid="query-split-toggle"
                >
                  <Columns2 className="h-4 w-4" />
                </Button>
              ) : null}

              {historyDock ? (
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={toggleHistory}
                  aria-label={t("query.history.toggle", "Toggle history")}
                  aria-pressed={historyOpen}
                  data-testid="query-history-toggle"
                >
                  <PanelRight className="h-4 w-4" />
                </Button>
              ) : null}

              <QuerySettingsSheet
                settings={{
                  stream: querySettings.stream,
                  topK: querySettings.topK,
                  temperature: querySettings.temperature,
                  maxTokens: querySettings.maxTokens,
                  systemPrompt: querySettings.systemPrompt,
                  fullChunkContent: querySettings.fullChunkContent,
                }}
                onSettingsChange={(updates) => setQuerySettings(updates)}
                providerModel={
                  querySettings.provider && querySettings.model
                    ? `${querySettings.provider}/${querySettings.model}`
                    : ""
                }
                documentFilter={querySettings.documentFilter}
                onDocumentFilterChange={(documentFilter) =>
                  setQuerySettings({ documentFilter })
                }
                scopedDocumentIds={scope.ids}
                onScopedDocumentIdsChange={scope.setDocumentIds}
              />
            </div>
          </header>

          {/* Single status live region — not on the message list (Q03/Q24) */}
          <div className="sr-only" role="status" aria-live="polite" aria-atomic>
            {statusAnnouncement}
          </div>

          <div className="flex-1 min-h-0 overflow-hidden relative">
            <ScrollArea ref={scrollRef} className="h-full">
              <div
                className="max-w-4xl lg:max-w-5xl mx-auto px-page pt-page pb-6"
                aria-label={t("query.messageList", "Conversation messages")}
              >
                {messages.length === 0 && !isLoading ? (
                  <QueryEmptyState
                    onSuggestionClick={handleSuggestionClick}
                    mode={querySettings.mode}
                  />
                ) : (
                  messages.map((message, index) => (
                    <ChatMessage
                      key={`${message.role}-${message.id}-${index}`}
                      message={{
                        ...message,
                        stopped:
                          message.stopped ||
                          (Boolean(pendingMessage?.id === message.id) &&
                            stage === "stopped"),
                      }}
                      onRegenerate={
                        message.role === "assistant" &&
                        index === messages.length - 1
                          ? handleRegenerate
                          : undefined
                      }
                      onRetry={
                        message.isError && index === messages.length - 1
                          ? handleRetry
                          : undefined
                      }
                      onContinue={
                        stage === "stopped" &&
                        index === messages.length - 1
                          ? handleRetry
                          : undefined
                      }
                      onEdit={
                        message.role === "user"
                          ? () => handleEditMessage(message.id)
                          : undefined
                      }
                      isLast={index === messages.length - 1}
                      stage={
                        index === messages.length - 1 ? stage : null
                      }
                      stageDetail={
                        index === messages.length - 1
                          ? stageDetail
                          : undefined
                      }
                    />
                  ))
                )}
                <div ref={scrollAnchorRef} className="h-32" />
              </div>
            </ScrollArea>

            {showJumpPill ? (
              <Button
                type="button"
                size="sm"
                variant="secondary"
                className="absolute bottom-4 left-1/2 -translate-x-1/2 gap-1 shadow-md z-20"
                onClick={jumpToLatest}
                data-testid="query-jump-latest"
              >
                <ArrowDown className="h-3.5 w-3.5" />
                {t("query.jumpToLatest", "Jump to latest")}
              </Button>
            ) : null}
          </div>

          <div className="px-page pt-2 pb-4 bg-gradient-to-t from-background via-background to-transparent shrink-0 relative z-10">
            <Composer
              input={input}
              slashMenu={
                slashMenuOpen ? (
                  <SlashMenu
                    onNewChat={() => {
                      handleNewConversation();
                      setSlashMenuOpen(false);
                    }}
                    onClose={() => setSlashMenuOpen(false)}
                  />
                ) : null
              }
              onInputChange={(value) => {
                handleInputChange(value);
                setSlashMenuOpen(value === "/");
              }}
              onSubmit={handleSubmit}
              onStop={handleStop}
              isStreaming={isStreaming}
              queuedMessage={queuedMessage}
              onClearQueue={clearQueue}
              onQueue={queueMessage}
              mode={querySettings.mode}
              onModeChange={(mode) => setQuerySettings({ mode })}
              searchImage={searchImage}
              scope={scope}
              attachedImages={attachedImages}
              onRemoveImage={removeImage}
              onImageButtonClick={() => imageInputRef.current?.click()}
              imageInputRef={imageInputRef}
              onImageInputChange={handleImageInputChange}
              maxImages={maxImages}
              onPaste={handlePaste}
              onDrop={handleDrop}
              onDragOver={handleDragOver}
              inputRef={inputRef}
              modelSlot={<ModelChip />}
              sendBlocked={choice.sendBlocked}
              sendBlockedHint={
                choice.empty
                  ? undefined
                  : t(
                      "query.model.pickHint",
                      "The workspace default isn’t available here. Choose a model below.",
                    )
              }
              scopePickerOpen={scopePickerOpen}
              onScopePickerOpenChange={setScopePickerOpen}
            />
          </div>
        </div>

        {/* SPEC-157: companion pane sits between chat and history */}
        <CompanionShell
          layout={layout}
          messages={messages}
          isMessagesLoading={isLoading}
          onQuote={quotePassage}
          onAskEntity={askEntity}
          onRestoreFocus={focusComposer}
        />

        {/* History: docked at xl+ when it fits (Q21); otherwise the sheet */}
        {historyDock ? (
          <ConversationHistoryPanelV2 maxWidth={companionOpen ? 340 : 500} />
        ) : null}
      </div>
    </TooltipProvider>
  );
}

export default QueryInterface;
