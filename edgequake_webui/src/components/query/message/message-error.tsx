"use client";

import { ErrorState } from "@/components/shared/error-state";
import { useTranslation } from "react-i18next";

interface MessageErrorProps {
  message: string;
  onRetry?: () => void;
}

/** Inline honest error with working Retry (SPEC-155 Q09). */
export function MessageError({ message, onRetry }: MessageErrorProps) {
  const { t } = useTranslation();
  return (
    <div className="mb-6" data-testid="query-message-error">
      <ErrorState
        title={t("query.errorTitle", "Couldn't complete that query")}
        description={humanise(message, t)}
        onRetry={onRetry}
        className="items-start text-left p-4"
      />
    </div>
  );
}

function humanise(
  raw: string,
  t: (key: string, fallback: string) => string,
): string {
  const lower = raw.toLowerCase();
  const timedOut =
    lower.includes("timeout") ||
    lower.includes("timed out") ||
    lower.includes("deadline exceeded") ||
    lower.includes("deadline_exceeded");
  if (timedOut && (lower.includes("storage") || lower.includes("database"))) {
    return t(
      "query.errorStorageTimeout",
      "Searching the knowledge base took too long. Try again in a moment.",
    );
  }
  if (lower.includes("network") || lower.includes("fetch")) {
    return t(
      "query.errorNetwork",
      "Network problem reaching the server. Check your connection and try again.",
    );
  }
  if (timedOut) {
    return t(
      "query.errorTimeout",
      "The model took too long to respond. Try again or switch mode.",
    );
  }
  if (lower.includes("rate") || lower.includes("429")) {
    return t(
      "query.errorRateLimit",
      "Rate limited by the LLM provider. Wait a moment and retry.",
    );
  }
  // Never dump stacks
  if (raw.length > 280 || raw.includes("\n    at ")) {
    return t(
      "query.errorGeneric",
      "Something went wrong while generating the answer.",
    );
  }
  return raw;
}
