"use client";

import { Progress } from "@/components/ui/progress";
import { useMigrateConversations } from "@/hooks/use-migrate-conversations";
import { AlertCircle, Loader2 } from "lucide-react";

export function MigrationBanner() {
  const migration = useMigrateConversations();

  // "checking" resolves within the same tick (no legacy data) in the common case;
  // rendering a banner for it would flash 37px and shift the whole panel (CLS).
  if (
    migration.status === "complete" ||
    migration.status === "pending" ||
    migration.status === "checking"
  ) {
    return null;
  }

  if (migration.status === "migrating") {
    const percent = Math.round((migration.progress / migration.total) * 100);
    return (
      <div className="bg-primary/10 border-b border-primary/20 px-4 py-3">
        <div className="flex items-center gap-2 mb-2">
          <Loader2 className="h-4 w-4 animate-spin text-primary" />
          <span className="text-sm font-medium">
            Migrating conversations... ({migration.progress}/{migration.total})
          </span>
        </div>
        <Progress value={percent} className="h-2" />
      </div>
    );
  }

  if (migration.status === "error") {
    return (
      <div className="bg-destructive/10 border-b border-destructive/20 px-4 py-2 flex items-center gap-2">
        <AlertCircle className="h-4 w-4 text-destructive" />
        <span className="text-sm text-destructive">
          Migration failed: {migration.error}
        </span>
      </div>
    );
  }

  return null;
}
