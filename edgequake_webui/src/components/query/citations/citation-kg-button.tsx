"use client";

import { Button } from "@/components/ui/button";
import { useWorkspaceCapabilities } from "@/hooks/use-workspace-capabilities";
import { cn } from "@/lib/utils";
import { Network } from "lucide-react";
import { useRouter } from "next/navigation";
import { useTranslation } from "react-i18next";

interface CitationKgButtonProps {
  documentId: string;
  className?: string;
}

/** Open Graph Studio filtered to this citation's document. */
export function CitationKgButton({
  documentId,
  className,
}: CitationKgButtonProps) {
  const { t } = useTranslation();
  const router = useRouter();
  const showsGraph = useWorkspaceCapabilities().shows_graph_ui;
  const id = documentId.trim();
  // SPEC-180: a workspace without a graph has no Graph Studio to open.
  if (!id || !showsGraph) return null;

  return (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className={cn(
        "h-5 w-5 shrink-0 text-muted-foreground hover:text-foreground",
        className,
      )}
      data-testid="citation-show-kg"
      aria-label={t("query.showOnGraph", "Show on graph")}
      title={t(
        "query.showOnGraphHint",
        "Highlight this document on the knowledge graph",
      )}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        router.push(`/graph?document=${encodeURIComponent(id)}`);
      }}
    >
      <Network className="h-3 w-3" aria-hidden />
    </Button>
  );
}
