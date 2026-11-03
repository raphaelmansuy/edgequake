"use client";

import {
  BookOpen,
  GitBranch,
  Lightbulb,
  MessageSquare,
  Search,
  Sparkles,
} from "lucide-react";
import { Fragment, memo, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";

import {
  SUGGESTION_SLOTS,
  getQueryEmptyCopy,
  getQueryEmptySuggestionKeys,
  isChatQueryMode,
  topicForCorpus,
} from "@/lib/query/query-empty-copy";
import type { QueryMode } from "@/types/query";
import { apiClient } from "@/lib/api/client";
import { getWorkspaceStats } from "@/lib/api/edgequake/workspaces";
import { useWorkspaceCapabilities } from "@/hooks/use-workspace-capabilities";
import { useQueryEmptyCorpus } from "@/hooks/use-query-empty-corpus";
import { useTenantStore } from "@/stores/use-tenant-store";

export interface QueryEmptyStateProps {
  onSuggestionClick?: (text: string) => void;
  /** Active query mode — Chat (bypass) uses chatbot copy, not KG copy. */
  mode?: QueryMode;
}

type EntityHit = { name?: string; label?: string; entity_type?: string };

/** Empty query chat state — no gradients (Q04/Q19); corpus-derived suggestions (Q20). */
export const QueryEmptyState = memo(function QueryEmptyState({
  onSuggestionClick,
  mode = "mix",
}: QueryEmptyStateProps) {
  const { t } = useTranslation();
  const isChat = isChatQueryMode(mode);
  const { shows_graph_ui: hasGraph } = useWorkspaceCapabilities();
  // SPEC-183: the suggestions must be questions this corpus can answer. A picture workspace was
  // being offered "What does the handbook say about refunds?" — unanswerable by construction.
  const corpus = useQueryEmptyCorpus(!isChat);
  const copy = getQueryEmptyCopy(mode, corpus);

  const selectedWorkspaceId = useTenantStore((s) => s.selectedWorkspaceId);

  const { data: stats } = useQuery({
    queryKey: ["workspace-stats-empty", selectedWorkspaceId],
    queryFn: () => getWorkspaceStats(selectedWorkspaceId!).catch(() => null),
    staleTime: 60_000,
    enabled: !isChat && !!selectedWorkspaceId,
  });

  const { data: topEntities } = useQuery({
    queryKey: ["empty-top-entities"],
    queryFn: async () => {
      try {
        const res = await apiClient<{ items?: EntityHit[] }>(
          "/graph/nodes/search?q=&limit=4",
        );
        return res.items ?? [];
      } catch {
        return [] as EntityHit[];
      }
    },
    staleTime: 60_000,
    enabled: !isChat && hasGraph,
  });

  const suggestionIcons = isChat
    ? [
        <MessageSquare key="0" className="h-4 w-4" />,
        <Lightbulb key="1" className="h-4 w-4" />,
        <Search key="2" className="h-4 w-4" />,
        <BookOpen key="3" className="h-4 w-4" />,
      ]
    : [
        <Search key="0" className="h-4 w-4" />,
        <Lightbulb key="1" className="h-4 w-4" />,
        <GitBranch key="2" className="h-4 w-4" />,
        <BookOpen key="3" className="h-4 w-4" />,
      ];

  const corpusSuggestions = useMemo(() => {
    if (isChat || !topEntities?.length) return null;
    return topEntities.slice(0, 4).map((e, i) => {
      const name = e.label || e.name || "";
      const display = name.replace(/_/g, " ");
      return {
        icon: suggestionIcons[i] ?? <Search className="h-4 w-4" />,
        text: t(
          "query.corpusSuggestion",
          "What do we know about {{name}}?",
          { name: display },
        ),
      };
    });
  }, [isChat, topEntities, t, suggestionIcons]);

  // SPEC-183: each corpus suggestion carries its own i18n key, so a translated question is not
  // looked up by position in a list whose length now depends on the corpus.
  const suggestionKeys = useMemo(
    () => getQueryEmptySuggestionKeys(mode, corpus),
    [mode, corpus],
  );
  // The translated string carries `{{topic}}` too, so it is substituted here as well — the
  // English fallback alone is not what renders. Without this the screen showed a raw
  // placeholder, which no test caught because both sides were individually correct.
  const topic = topicForCorpus(corpus);
  const suggestions =
    corpusSuggestions ??
    copy.suggestions.slice(0, SUGGESTION_SLOTS).map((text, i) => ({
      icon: suggestionIcons[i] ?? <Search className="h-4 w-4" />,
      text: t(suggestionKeys[i] ?? `${copy.keys.suggestions}.${i}`, text, { topic }),
    }));

  const hasDocs = (stats?.document_count ?? 0) > 0;
  const figures = hasGraph
    ? [
        { n: stats?.entity_count ?? 0, key: "query.stats.entities", text: "entities" },
        { n: stats?.relationship_count ?? 0, key: "query.stats.relationships", text: "relationships" },
      ]
    : [
        { n: stats?.document_count ?? 0, key: "query.stats.documents", text: "documents" },
        { n: stats?.chunk_count ?? 0, key: "query.stats.chunks", text: "passages" },
      ];

  return (
    // Top-anchored with a viewport-relative gap: sits low on tall screens and
    // starts at the very top (scrollable) on short ones — never clipped (SPEC-168).
    <div className="flex flex-col items-center px-4 pb-6 pt-[clamp(0rem,5vh,3.5rem)]">
      <div
        className="mb-4 flex h-12 w-12 items-center justify-center rounded-2xl bg-primary text-primary-foreground"
        aria-hidden="true"
      >
        {isChat ? (
          <MessageSquare className="h-6 w-6" />
        ) : (
          <Sparkles className="h-6 w-6" />
        )}
      </div>

      <h2 className="mb-2 text-center text-2xl font-semibold tracking-tight">
        {t(copy.keys.title, copy.title)}
      </h2>
      <p className="mb-5 max-w-lg text-center leading-relaxed text-muted-foreground">
        {t(copy.keys.description, copy.description)}
      </p>

      {/* Reserved slot: the pill fades in without moving the content below it. */}
      {!isChat && (
        <div
          className={`mb-6 flex h-10 items-center gap-4 rounded-full border bg-muted/30 px-6 transition-opacity ${stats ? "opacity-100" : "opacity-0"}`}
          role="status"
          aria-hidden={stats ? undefined : true}
        >
          {figures.map((f, i) => (
            <Fragment key={f.key}>
              {i > 0 ? <span className="w-px h-4 bg-border" aria-hidden /> : null}
              <span className="text-sm">
                <span className="font-medium">{f.n}</span>{" "}
                <span className="text-muted-foreground">{t(f.key, f.text)}</span>
              </span>
            </Fragment>
          ))}
        </div>
      )}

      {!isChat && !hasDocs && stats ? (
        <p className="text-sm text-muted-foreground mb-6 text-center max-w-md">
          {hasGraph
            ? t(
                "query.emptyNoDocs",
                "No documents yet — upload documents to ground answers in your knowledge graph.",
              )
            : t(
                "query.normalEmptyNoDocs",
                "No documents yet — upload documents and I will answer from their passages.",
              )}
        </p>
      ) : null}

      {onSuggestionClick && (
        <div className="w-full max-w-2xl space-y-3">
          <p className="mb-2 text-center text-sm font-medium text-muted-foreground">
            {t("query.tryAsking", "Try asking:")}
          </p>
          <div
            className="grid grid-cols-1 md:grid-cols-2 gap-2"
            role="list"
            aria-label={t("query.suggestedQueries", "Suggested queries")}
          >
            {suggestions.map((suggestion, i) => (
              <button
                key={i}
                type="button"
                onClick={() => onSuggestionClick(suggestion.text)}
                className="group flex min-h-[60px] items-start gap-3 text-left px-4 py-3 rounded-xl border bg-card hover:bg-muted/50 hover:border-primary/30 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
                role="listitem"
                aria-label={suggestion.text}
              >
                <div
                  className="p-1.5 rounded-lg bg-muted group-hover:bg-primary/10 transition-colors shrink-0"
                  aria-hidden="true"
                >
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
