/**
 * SPEC-185 — "This workspace stores text only", shown above the upload drop target.
 *
 * WHY it is no longer inside the dropzone:
 *   The dropzone is a flex row when the zone is short and wide. The notice sat in that row as a
 *   sibling, took 897px of a 963px line, and squeezed the parser/vision selects to 0px — so the
 *   upload controls vanished behind a message about a case that had not happened yet. At a narrow
 *   column the same paragraph wrapped to 192px inside a 97px zone and was clipped to nothing.
 *   It is a fact about the workspace, not an upload instruction, so it belongs beside the drop
 *   target rather than inside it.
 *
 * WHY it starts collapsed to one line:
 *   The condition is rare (a picture dropped on a text-only workspace) and the fact is permanent,
 *   so four lines of prose above the primary action is the definition of clunky. One line costs
 *   ~28px and still states the whole point; the reason and the fix are one click away, and
 *   screen readers get them through `aria-expanded` + `aria-controls`.
 */
"use client";

import { Button } from "@/components/ui/button";
import { useTextOnlyNoticeDismissed } from "@/hooks/use-text-only-notice-dismissed";
import type { ImageCapability } from "@/lib/documents/image-upload-guidance";
import { cn } from "@/lib/utils";
import { ChevronDown, ImageOff, X } from "lucide-react";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";

export interface WorkspaceTextOnlyNoticeProps {
  /** The capability the server stated. Absent means unknown, and nothing is said. */
  capability: ImageCapability;
  /** Scopes the dismissal. Without it the note still shows but cannot be dismissed. */
  workspaceId?: string | null;
  className?: string;
}

export function WorkspaceTextOnlyNotice({
  capability,
  workspaceId,
  className,
}: WorkspaceTextOnlyNoticeProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const { dismissed, dismiss } = useTextOnlyNoticeDismissed(workspaceId);
  const bodyId = useId();

  // Only an explicit `false` is a refusal. Unknown must stay silent: guessing would
  // hide an explanation on a workspace that can store pictures.
  if (capability?.embeds_images !== false || dismissed) return null;

  return (
    <div
      role="status"
      data-testid="dropzone-text-only-notice"
      data-expanded={expanded ? "true" : "false"}
      className={cn(
        "@container/note mx-3 mt-2 shrink-0 rounded-md border border-amber-300/70 bg-amber-50/70 px-2 py-1 text-[11.5px] leading-snug text-amber-900 dark:border-amber-900 dark:bg-amber-950/30 dark:text-amber-200",
        className,
      )}
    >
      <div className="flex items-center gap-1.5">
        <ImageOff aria-hidden className="h-3.5 w-3.5 shrink-0" />
        <button
          type="button"
          onClick={() => setExpanded((open) => !open)}
          aria-expanded={expanded}
          aria-controls={bodyId}
          className="group/note flex min-w-0 flex-1 items-center gap-1.5 rounded-sm text-left font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-amber-500/60"
          data-testid="dropzone-text-only-notice-toggle"
        >
          {/* A 216px column truncated the full sentence to "This workspace st…". The short
              form says the same thing in the space that is actually available. */}
          <span className="truncate @[15rem]/note:hidden">
            {t("documents.upload.textOnlyTitleShort", "Text only")}
          </span>
          <span className="hidden truncate @[15rem]/note:inline">
            {t("documents.upload.textOnlyTitle", "This workspace stores text only")}
          </span>
          {/* Separated from the sentence and underlined on hover, so it reads as "open this",
              not as a caret that belongs to the words. */}
          <span className="ml-auto flex shrink-0 items-center gap-0.5 text-[10.5px] font-normal opacity-80 group-hover/note:opacity-100">
            <span className="underline decoration-amber-700/40 underline-offset-2 dark:decoration-amber-300/40">
              {expanded
                ? t("documents.upload.textOnlyHide", "Hide")
                : t("documents.upload.textOnlyWhy", "Why")}
            </span>
            <ChevronDown
              aria-hidden
              className={cn(
                "h-3.5 w-3.5 transition-transform duration-150",
                expanded && "rotate-180",
              )}
            />
          </span>
        </button>
        {workspaceId ? (
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            onClick={dismiss}
            className="-my-1 -mr-1 shrink-0 text-amber-900/70 hover:bg-amber-100 hover:text-amber-900 dark:text-amber-200/70 dark:hover:bg-amber-900/40 dark:hover:text-amber-100"
            aria-label={t("documents.upload.dismissTextOnly", "Dismiss this note")}
            title={t("documents.upload.dismissTextOnly", "Dismiss this note")}
            data-testid="dropzone-text-only-notice-dismiss"
          >
            <X aria-hidden className="h-3.5 w-3.5" />
          </Button>
        ) : null}
      </div>
      {/* Kept in the DOM only while open, so a collapsed note costs one line and no
          dangling reference for `aria-controls` to point at. */}
      {expanded ? (
        <p
          id={bodyId}
          className="mt-1.5 pl-5"
          data-testid="dropzone-text-only-notice-body"
        >
          {t(
            "documents.upload.textOnlyBody",
            "Evidence is set once, when a workspace is created. Create a new workspace with Evidence = “Text and images”, then upload pictures there.",
          )}
        </p>
      ) : null}
    </div>
  );
}

export default WorkspaceTextOnlyNotice;
