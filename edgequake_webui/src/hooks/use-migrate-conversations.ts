"use client";

/**
 * @module use-migrate-conversations
 * @description Hook for migrating localStorage conversations to server.
 * Handles one-time migration with progress tracking.
 *
 * @implements UC0411 - User migrates local conversations to server
 * @implements FEAT0643 - One-time localStorage migration
 * @implements FEAT0644 - Migration progress tracking
 *
 * @enforces BR0628 - Migration runs once per browser
 * @enforces BR0629 - Failed migration preserves local data
 */

import type { LocalStorageConversation } from "@/types";
import { useEffect, useRef, useState } from "react";
import { useImportConversations } from "./use-conversations";

const MIGRATION_KEY = "edgequake-conversations-migrated";

interface MigrationState {
  status: "pending" | "checking" | "migrating" | "complete" | "error";
  progress: number;
  total: number;
  error?: string;
}

export function useMigrateConversations() {
  const [state, setState] = useState<MigrationState>({
    status: "pending",
    progress: 0,
    total: 0,
  });

  const importMutation = useImportConversations();
  const hasChecked = useRef(false);

  useEffect(() => {
    const checkAndMigrate = async () => {
      // Prevent duplicate runs
      if (hasChecked.current) return;
      hasChecked.current = true;

      // Check if already migrated
      if (typeof window === "undefined") return;
      if (localStorage.getItem(MIGRATION_KEY)) {
        setState({ status: "complete", progress: 0, total: 0 });
        return;
      }

      setState({ status: "checking", progress: 0, total: 0 });

      // Check for old conversations
      const oldData = localStorage.getItem("edgequake-conversations");
      if (!oldData) {
        localStorage.setItem(MIGRATION_KEY, "true");
        setState({ status: "complete", progress: 0, total: 0 });
        return;
      }

      try {
        const parsed = JSON.parse(oldData);
        const conversations: LocalStorageConversation[] =
          parsed.state?.conversations ?? [];

        if (conversations.length === 0) {
          localStorage.setItem(MIGRATION_KEY, "true");
          setState({ status: "complete", progress: 0, total: 0 });
          return;
        }

        setState({
          status: "migrating",
          progress: 0,
          total: conversations.length,
        });

        // Import in batches of 10
        const batchSize = 10;
        for (let i = 0; i < conversations.length; i += batchSize) {
          const batch = conversations.slice(i, i + batchSize);
          await importMutation.mutateAsync({ conversations: batch });
          setState((prev) => ({
            ...prev,
            progress: Math.min(i + batchSize, conversations.length),
          }));
        }

        // Mark as migrated
        localStorage.setItem(MIGRATION_KEY, "true");
        // Optionally clear old data
        // localStorage.removeItem('edgequake-conversations');

        setState({
          status: "complete",
          progress: conversations.length,
          total: conversations.length,
        });
      } catch (error) {
        setState({
          status: "error",
          progress: 0,
          total: 0,
          error: error instanceof Error ? error.message : "Unknown error",
        });
      }
    };

    checkAndMigrate();
  }, [importMutation]);

  return state;
}
