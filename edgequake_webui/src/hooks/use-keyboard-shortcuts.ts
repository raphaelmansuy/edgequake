"use client";

import { useRouter } from "next/navigation";
import { useEffect } from "react";

/**
 * Global keyboard shortcuts for EdgeQuake WebUI
 *
 * Shortcuts:
 * - Cmd/Ctrl + K: Open command palette (future)
 * - Cmd/Ctrl + G: Go to Graph
 * - Cmd/Ctrl + D: Go to Documents
 * - Cmd/Ctrl + Q: Go to Query
 * - Cmd/Ctrl + ,: Go to Settings
 * - Escape: Close modals/dialogs
 */
export function useKeyboardShortcuts() {
  const router = useRouter();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Skip if user is typing in an input
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        return;
      }

      const isMeta = e.metaKey || e.ctrlKey;

      // Cmd/Ctrl + G: Go to Graph
      if (isMeta && e.key === "g") {
        e.preventDefault();
        router.push("/graph");
      }

      // Cmd/Ctrl + D: Go to Documents
      if (isMeta && e.key === "d") {
        e.preventDefault();
        router.push("/documents");
      }

      // Cmd/Ctrl + Q: Go to Query (use Shift to avoid quit conflict)
      if (isMeta && e.shiftKey && e.key === "Q") {
        e.preventDefault();
        router.push("/query");
      }

      // Cmd/Ctrl + ,: Go to Settings
      if (isMeta && e.key === ",") {
        e.preventDefault();
        router.push("/settings");
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [router]);
}
