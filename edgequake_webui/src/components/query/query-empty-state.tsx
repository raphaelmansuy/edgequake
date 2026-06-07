"use client";

import { BookOpen, GitBranch, Lightbulb, Search, Sparkles } from "lucide-react";
import { memo } from "react";
import { useTranslation } from "react-i18next";

export interface QueryEmptyStateProps {
  onSuggestionClick?: (text: string) => void;
  graphStats?: { entities: number; relationships: number; types: number };
}

/** Empty query chat state with suggestions and optional graph stats (SPEC-017 UI-P3-005). */
export const QueryEmptyState = memo(function QueryEmptyState({
  onSuggestionClick,
  graphStats,
}: QueryEmptyStateProps) {
  const { t } = useTranslation();

  const suggestions = [
    {
      icon: <Search className="h-4 w-4" />,
      text: t(
        "query.suggestions.0",
        "What are the main entities in my knowledge graph?",
      ),
      category: "exploration",
    },
    {
      icon: <Lightbulb className="h-4 w-4" />,
      text: t(
        "query.suggestions.1",
        "Summarize the key relationships between documents",
      ),
      category: "summary",
    },
    {
      icon: <GitBranch className="h-4 w-4" />,
      text: t(
        "query.suggestions.2",
        "Find connections between people and organizations",
      ),
      category: "relationships",
    },
    {
      icon: <BookOpen className="h-4 w-4" />,
      text: t(
        "query.suggestions.3",
        "What topics are covered in my documents?",
      ),
      category: "topics",
    },
  ];

  const hasData =
    graphStats && (graphStats.entities > 0 || graphStats.relationships > 0);

  return (
    <div className="flex flex-col items-center justify-center h-full py-12 px-4 motion-safe:animate-fade-in-up">
      <div className="relative mb-8" aria-hidden="true">
        <div className="absolute inset-0 bg-gradient-to-r from-primary/40 to-primary/60 rounded-2xl blur-2xl opacity-20 motion-safe:animate-pulse-soft" />
        <div className="relative bg-gradient-to-br from-primary/80 to-primary rounded-2xl p-5 shadow-lg">
          <Sparkles className="h-10 w-10 text-primary-foreground" />
        </div>
      </div>

      <h2 className="text-2xl font-bold mb-2 text-center">
        {t("query.emptyTitle", "Ask about your knowledge graph")}
      </h2>
      <p className="text-muted-foreground text-center mb-8 max-w-lg leading-relaxed">
        {t(
          "query.emptyDescription",
          "I can help you explore entities, find connections, and uncover insights from your documents.",
        )}
      </p>

      {hasData && (
        <div
          className="flex items-center gap-4 mb-8 px-6 py-3 bg-muted/30 rounded-full border border-border/50"
          role="status"
          aria-label={`${graphStats.entities} entities, ${graphStats.relationships} relationships, ${graphStats.types} types`}
        >
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-green-500" aria-hidden="true" />
            <span className="text-sm font-medium">{graphStats.entities}</span>
            <span className="text-xs text-muted-foreground">entities</span>
          </div>
          <div className="w-px h-4 bg-border" aria-hidden="true" />
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-amber-500" aria-hidden="true" />
            <span className="text-sm font-medium">{graphStats.relationships}</span>
            <span className="text-xs text-muted-foreground">relationships</span>
          </div>
          <div className="w-px h-4 bg-border" aria-hidden="true" />
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-blue-500" aria-hidden="true" />
            <span className="text-sm font-medium">{graphStats.types}</span>
            <span className="text-xs text-muted-foreground">types</span>
          </div>
        </div>
      )}

      {onSuggestionClick && (
        <div className="w-full max-w-2xl space-y-3">
          <p className="text-sm font-medium text-muted-foreground text-center mb-3">
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
                onClick={() => onSuggestionClick(suggestion.text)}
                className="group flex items-start gap-3 text-left px-4 py-3.5 rounded-xl border bg-card hover:bg-muted/50 hover:border-primary/30 transition-all duration-200 hover:shadow-sm hover:-translate-y-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
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
