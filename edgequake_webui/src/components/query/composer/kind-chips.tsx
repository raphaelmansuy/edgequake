"use client";

import { DOC_KINDS, type DocKind } from "@/constants/rag-profile";
import { cn } from "@/lib/utils";
import { useTranslation } from "react-i18next";

const KIND_LABELS: Record<DocKind, { key: string; fallback: string }> = {
  pdf: { key: "query.kinds.pdf", fallback: "PDFs" },
  image: { key: "query.kinds.image", fallback: "Images" },
  markdown: { key: "query.kinds.markdown", fallback: "Markdown" },
  text: { key: "query.kinds.text", fallback: "Text" },
  office: { key: "query.kinds.office", fallback: "Office" },
};

interface KindChipsProps {
  selected: readonly DocKind[];
  onToggle: (kind: DocKind) => void;
  onClear: () => void;
}

/**
 * SPEC-180 — which kinds of document the answer may draw on. Multi-select; none selected
 * means all. "All" is a real chip so the empty state reads as a choice, not as a gap.
 */
export function KindChips({ selected, onToggle, onClear }: KindChipsProps) {
  const { t } = useTranslation();
  const chip = (active: boolean) =>
    cn(
      "inline-flex h-7 items-center rounded-full px-2.5 text-xs font-medium transition-colors",
      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40",
      active
        ? "bg-primary/[0.1] text-foreground ring-1 ring-primary/25"
        : "text-muted-foreground ring-1 ring-border/70 hover:bg-muted hover:text-foreground",
    );

  return (
    <div
      role="group"
      aria-label={t("query.kinds.groupLabel", "Document kinds")}
      className="flex flex-wrap items-center gap-1.5 px-3 pt-3"
      data-testid="query-kind-chips"
    >
      <button
        type="button"
        aria-pressed={selected.length === 0}
        onClick={onClear}
        className={chip(selected.length === 0)}
        data-testid="query-kind-all"
      >
        {t("query.kinds.all", "All")}
      </button>
      {DOC_KINDS.map((kind) => {
        const active = selected.includes(kind);
        const label = KIND_LABELS[kind];
        return (
          <button
            key={kind}
            type="button"
            aria-pressed={active}
            onClick={() => onToggle(kind)}
            className={chip(active)}
            data-testid={`query-kind-${kind}`}
          >
            {t(label.key, label.fallback)}
          </button>
        );
      })}
    </div>
  );
}
