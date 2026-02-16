/**
 * @module WorkspaceDeeplinkStates
 * @description Shared UI components for workspace deeplink loading and error states.
 * Eliminates duplicated JSX across /w/[slug]/ pages.
 *
 * @implements SPEC-032: Focus 6 - Deeplinks to workspace
 *
 * WHY: Each deeplink page had identical loading spinner and 404 error UI.
 * Extracting these into shared components follows SRP (page handles routing,
 * component handles presentation) and DRY (one source of truth for UI).
 */
import { Loader2 } from "lucide-react";

interface WorkspaceLoadingProps {
  /** What is being loaded, shown to the user. */
  context?: string;
}

/**
 * Loading spinner for workspace resolution.
 */
export function WorkspaceLoading({
  context = "workspace",
}: WorkspaceLoadingProps) {
  return (
    <div className="flex items-center justify-center h-full">
      <div className="text-center">
        <Loader2 className="h-8 w-8 animate-spin mx-auto text-muted-foreground mb-3" />
        <p className="text-sm text-muted-foreground">
          Loading {context}...
        </p>
      </div>
    </div>
  );
}

interface WorkspaceNotFoundProps {
  /** The slug that was not found. */
  slug: string;
  /** Where to redirect the user. */
  fallbackHref?: string;
  /** Label for the fallback link. */
  fallbackLabel?: string;
}

/**
 * 404 error state for workspace not found.
 */
export function WorkspaceNotFound({
  slug,
  fallbackHref = "/workspace",
  fallbackLabel = "Go to Workspace Settings",
}: WorkspaceNotFoundProps) {
  return (
    <div className="flex items-center justify-center h-full">
      <div className="text-center">
        <h1 className="text-2xl font-semibold mb-2">Workspace Not Found</h1>
        <p className="text-muted-foreground mb-4">
          The workspace &quot;{slug}&quot; does not exist or you don&apos;t have
          access.
        </p>
        <a href={fallbackHref} className="text-primary hover:underline">
          {fallbackLabel}
        </a>
      </div>
    </div>
  );
}

/**
 * Generic redirect-in-progress spinner.
 */
export function WorkspaceRedirecting() {
  return (
    <div className="flex items-center justify-center h-full">
      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
    </div>
  );
}
