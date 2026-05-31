"use client";

import { CheckCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

export function WorkspaceStatusFooter() {
  const { t } = useTranslation();

  return (
    <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
      <CheckCircle className="h-4 w-4 text-green-500" />
      {t(
        "workspace.statusReady",
        "Workspace ready for queries and document ingestion",
      )}
    </div>
  );
}
