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
  /*
   * SPEC-185 — a permission failure used to be reported as "Something went wrong".
   *
   * `PERMISSION_DENIED` arrives as a long JSON body, so the length guard below swallowed the one
   * useful fact. That hid a real, fixable misconfiguration: the model's Google identity had no
   * access to the configured project, which reads as a generic outage and leaves the operator with
   * nothing to act on. Naming it turns an unexplained failure into a one-line fix.
   */
  if (
    lower.includes("permission_denied") ||
    lower.includes("permission '") ||
    lower.includes("permission ") ||
    lower.includes("403")
  ) {
    return t(
      "query.errorPermission",
      "The model provider refused the request: its credentials are not allowed to use this project or model. Ask an administrator to check the provider credentials.",
    );
  }
  if (lower.includes("unauthenticated") || lower.includes("401")) {
    return t(
      "query.errorUnauthenticated",
      "The model provider rejected the credentials. They may have expired — an administrator can reconnect the provider.",
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
