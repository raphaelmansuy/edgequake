"use client";

import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import type { useQueryScope } from "@/hooks/use-query-scope";
import {
  composerPhaseFromSession,
  composerShowsSend,
  composerShowsStop,
} from "@/lib/query/composer-ui-machine";
import type { AttachedImage } from "@/lib/query/query-interface-types";
import { cn } from "@/lib/utils";
import type { DocumentSearchItem, QueryMode } from "@/types";
import { ImagePlus, Send, StopCircle, X } from "lucide-react";
import {
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
  type FormEvent,
  type RefObject,
} from "react";
import { useTranslation } from "react-i18next";
import { DocumentPickerPopover } from "../document-picker-popover";
import { AttachmentTray } from "./attachment-tray";
import {
  MENTION_LISTBOX_ID,
  MentionMenu,
  mentionOptionId,
} from "./mention-menu";
import { ModeMenu } from "./mode-menu";
import { ScopeChips } from "./scope-chips";
import { useComposerKeys } from "./use-composer-keys";
import { useMentionMenu } from "./use-mention-menu";

export type QueryScope = ReturnType<typeof useQueryScope>;

const MAX_TEXTAREA_PX = 200;

export interface ComposerProps {
  input: string;
  onInputChange: (value: string) => void;
  onSubmit: () => void | Promise<void>;
  onStop: () => void;
  isStreaming: boolean;
  /** When streaming, Enter queues instead of blocking */
  queuedMessage: string | null;
  onClearQueue: () => void;
  onQueue: (text: string) => void;
  mode: QueryMode;
  onModeChange: (mode: QueryMode) => void;
  /** Document scope (ids + titles) shared by `@`, the picker and chips. */
  scope: QueryScope;
  attachedImages: AttachedImage[];
  onRemoveImage: (index: number) => void;
  onImageButtonClick: () => void;
  imageInputRef: RefObject<HTMLInputElement | null>;
  onImageInputChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  maxImages: number;
  onPaste: (e: React.ClipboardEvent) => void;
  onDrop: (e: React.DragEvent) => void;
  onDragOver: (e: React.DragEvent) => void;
  inputRef: RefObject<HTMLTextAreaElement | null>;
  /** Optional model chip slot */
  modelSlot?: React.ReactNode;
  settingsSlot?: React.ReactNode;
  scopePickerOpen?: boolean;
  onScopePickerOpenChange?: (open: boolean) => void;
  slashMenu?: React.ReactNode;
}

export function Composer({
  input,
  onInputChange,
  onSubmit,
  onStop,
  isStreaming,
  queuedMessage,
  onClearQueue,
  onQueue,
  mode,
  onModeChange,
  scope,
  attachedImages,
  onRemoveImage,
  onImageButtonClick,
  imageInputRef,
  onImageInputChange,
  maxImages,
  onPaste,
  onDrop,
  onDragOver,
  inputRef,
  modelSlot,
  settingsSlot,
  scopePickerOpen,
  onScopePickerOpenChange,
  slashMenu,
}: ComposerProps) {
  const { t } = useTranslation();
  const { onCompositionStart, onCompositionEnd, isImeBlocked } =
    useComposerKeys();
  const dropActiveRef = useRef(false);
  const [caret, setCaret] = useState(0);

  const phase = composerPhaseFromSession({
    isStreaming,
    queuedMessage,
    isError: false,
  });
  const showStop = composerShowsStop(phase);
  const showSend = composerShowsSend(phase);

  // Keep the textarea height in sync with its content (grows, and shrinks after send).
  useLayoutEffect(() => {
    const el = inputRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, MAX_TEXTAREA_PX)}px`;
    // Programmatic edits (global `@`, edit-and-resubmit) move the caret without
    // firing onChange — re-read it so the mention menu stays in sync.
    setCaret(el.selectionStart ?? input.length);
  }, [input, inputRef]);

  const handlePick = useCallback(
    (doc: DocumentSearchItem, next: { text: string; caret: number }) => {
      scope.addDocument({ id: doc.id, title: doc.title });
      onInputChange(next.text);
      requestAnimationFrame(() => {
        const el = inputRef.current;
        el?.focus();
        el?.setSelectionRange(next.caret, next.caret);
        setCaret(next.caret);
      });
    },
    [inputRef, onInputChange, scope],
  );

  const mention = useMentionMenu({
    text: input,
    caret,
    scopedIds: scope.ids,
    onPick: handlePick,
  });
  const activeOptionId = mention.visible
    ? mention.items[mention.activeIndex]
      ? mentionOptionId(mention.items[mention.activeIndex]!.id)
      : undefined
    : undefined;

  const handleSubmit = (e?: FormEvent) => {
    e?.preventDefault();
    const text = input.trim();
    if (!text) return;
    if (phase === "streaming") {
      onQueue(text);
      onInputChange("");
      return;
    }
    void onSubmit();
  };

  const scopeLabel =
    scope.ids.length === 0
      ? t("query.scope.allDocs", "All docs")
      : t("query.scope.count", "{{count}} docs", { count: scope.ids.length });

  return (
    <form
      onSubmit={handleSubmit}
      className="relative max-w-4xl lg:max-w-5xl mx-auto"
      role="form"
      aria-label={t("query.form", "Query form")}
    >
      {/* Floating menus sit above the shell without pushing layout. */}
      {mention.visible || slashMenu ? (
        <div className="absolute inset-x-0 bottom-full z-30 mb-2">
          {mention.visible ? (
            <MentionMenu
              query={mention.query}
              items={mention.items}
              activeIndex={mention.activeIndex}
              isLoading={mention.isLoading}
              onHover={mention.setActiveIndex}
              onPick={mention.pick}
            />
          ) : (
            slashMenu
          )}
        </div>
      ) : null}

      {queuedMessage ? (
        <div
          className="mb-2 flex items-center gap-2 rounded-xl bg-muted/50 px-3 py-1.5 text-xs ring-1 ring-border/50"
          data-testid="query-queued-chip"
        >
          <span className="text-muted-foreground shrink-0">
            {t("query.queued", "Queued")}
          </span>
          <span className="truncate flex-1">{queuedMessage}</span>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="h-6 w-6 rounded-full"
            onClick={onClearQueue}
            aria-label={t("query.clearQueue", "Clear queued message")}
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      ) : null}

      <AttachmentTray images={attachedImages} onRemove={onRemoveImage} />

      <input
        ref={imageInputRef}
        type="file"
        accept="image/jpeg,image/png,image/gif,image/webp"
        multiple
        className="sr-only"
        aria-label={t("query.attachImages", "Attach images")}
        onChange={onImageInputChange}
      />

      <div
        className={cn(
          // Floating shell — ambient depth only (no stroke/ring = less “clunky” edge)
          "rounded-3xl bg-background overflow-hidden",
          "shadow-[0_2px_8px_rgba(15,23,42,0.04),0_12px_40px_-10px_rgba(15,23,42,0.12)]",
          "dark:shadow-[0_2px_8px_rgba(0,0,0,0.35),0_16px_48px_-12px_rgba(0,0,0,0.55)]",
          "transition-[box-shadow] duration-200 ease-out",
          // Focus: soft lift + brand wash (no hard ring stroke)
          "focus-within:shadow-[0_2px_10px_rgba(15,23,42,0.06),0_16px_48px_-10px_rgba(15,23,42,0.16),0_0_0_3px_color-mix(in_oklch,var(--primary)_12%,transparent)]",
          "dark:focus-within:shadow-[0_2px_10px_rgba(0,0,0,0.4),0_18px_52px_-12px_rgba(0,0,0,0.6),0_0_0_3px_color-mix(in_oklch,var(--primary)_16%,transparent)]",
        )}
        onDrop={(e) => {
          dropActiveRef.current = false;
          onDrop(e);
        }}
        onDragOver={(e) => {
          dropActiveRef.current = true;
          onDragOver(e);
        }}
        onDragLeave={() => {
          dropActiveRef.current = false;
        }}
        data-testid="query-composer"
      >
        <ScopeChips
          ids={scope.ids}
          titles={scope.titles}
          onRemove={scope.removeDocument}
        />

        <Textarea
          ref={inputRef}
          value={input}
          onChange={(e) => {
            onInputChange(e.target.value);
            setCaret(e.target.selectionStart ?? e.target.value.length);
          }}
          onSelect={(e) => setCaret(e.currentTarget.selectionStart ?? 0)}
          onPaste={onPaste}
          placeholder={t(
            "query.placeholderHint",
            "Ask a question…  type @ to focus on a document",
          )}
          className={cn(
            "query-input min-h-[52px] max-h-[200px] resize-none [scrollbar-gutter:stable]",
            "border-0 border-none bg-transparent shadow-none rounded-none ring-0 outline-none",
            "focus-visible:border-0 focus-visible:ring-0 focus-visible:shadow-none focus-visible:outline-none",
            "px-4 pt-3.5 pb-1 text-[15px] leading-relaxed",
            "placeholder:text-muted-foreground/55",
          )}
          rows={1}
          onCompositionStart={onCompositionStart}
          onCompositionEnd={onCompositionEnd}
          onKeyDown={(event) => {
            if (isImeBlocked(event.nativeEvent)) {
              if (event.key === "Enter") event.preventDefault();
              return;
            }
            if (mention.handleKeyDown(event)) return;
            if (event.key === "Escape" && isStreaming) {
              event.preventDefault();
              onStop();
              return;
            }
            if (event.key !== "Enter" || event.shiftKey) return;
            event.preventDefault();
            handleSubmit();
          }}
          // Always enabled (Q01/Q07) — never disable while streaming
          aria-label={t("query.placeholder", "Ask a question")}
          aria-describedby="query-hint"
          role="combobox"
          aria-autocomplete="list"
          aria-expanded={mention.visible}
          aria-controls={mention.visible ? MENTION_LISTBOX_ID : undefined}
          aria-activedescendant={activeOptionId}
        />
        <span id="query-hint" className="sr-only">
          {t(
            "query.hint",
            "Press Enter to send, Shift+Enter for new line, @ to focus on a document",
          )}
        </span>

        <div className="flex items-center gap-1 px-2.5 pb-2.5 pt-0.5">
          <div className="flex items-center gap-0.5 min-w-0 flex-1">
            <Button
              type="button"
              size="sm"
              variant="ghost"
              onClick={onImageButtonClick}
              disabled={attachedImages.length >= maxImages}
              className="h-8 w-8 p-0 text-muted-foreground hover:text-foreground"
              aria-label={t("query.attachImages", "Attach images")}
              title={t("query.attachImages", "Attach images")}
            >
              <ImagePlus className="h-4 w-4" aria-hidden="true" />
            </Button>

            <DocumentPickerPopover
              selectedIds={scope.ids}
              onSelectionChange={scope.setDocumentIds}
              onDocumentTitle={scope.rememberTitle}
              open={scopePickerOpen}
              onOpenChange={onScopePickerOpenChange}
              trigger={
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-8 px-2 text-xs gap-1 max-w-[8rem] text-muted-foreground hover:text-foreground"
                  data-testid="query-scope-chip"
                >
                  <span className="truncate">{scopeLabel}</span>
                </Button>
              }
            />

            <ModeMenu value={mode} onChange={onModeChange} compact />

            {modelSlot}
          </div>

          <div className="flex items-center gap-1 shrink-0">
            {settingsSlot}
            {showStop ? (
              <Button
                type="button"
                size="sm"
                variant="secondary"
                onClick={onStop}
                className="h-8 gap-1 rounded-full"
                aria-label={t("query.stop", "Stop generating")}
                data-testid="query-stop"
              >
                <StopCircle className="h-4 w-4" aria-hidden="true" />
                {t("query.stop", "Stop")}
              </Button>
            ) : showSend ? (
              <Button
                type="submit"
                size="sm"
                disabled={!input.trim()}
                className={cn(
                  "h-8 w-8 p-0 rounded-full",
                  "disabled:opacity-30 disabled:shadow-none",
                )}
                aria-label={t("query.submit", "Send message")}
                data-testid="query-send"
              >
                <Send className="h-3.5 w-3.5" aria-hidden="true" />
              </Button>
            ) : null}
          </div>
        </div>
      </div>
    </form>
  );
}
