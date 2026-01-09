"use client";

/**
 * Share Dialog Component
 *
 * Provides a dialog for sharing conversations via a public link.
 */

import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
    useShareConversation,
    useUnshareConversation,
} from "@/hooks/use-conversations";
import type { ConversationWithMessages } from "@/types";
import {
    Check,
    Copy,
    ExternalLink,
    Link,
    Loader2,
    Share2,
    Trash2,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

// ============================================================================
// Types
// ============================================================================

interface ShareDialogProps {
  conversation: ConversationWithMessages | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

// ============================================================================
// Component
// ============================================================================

export function ShareDialog({
  conversation,
  open,
  onOpenChange,
}: ShareDialogProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const shareConversation = useShareConversation();
  const unshareConversation = useUnshareConversation();

  // Reset copied state when dialog opens
  useEffect(() => {
    if (open) {
      // Intentional: Resetting UI state when dialog opens
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setCopied(false);
    }
  }, [open]);

  // Generate share URL from share_id
  const shareUrl = conversation?.share_id
    ? `${window.location.origin}/shared/${conversation.share_id}`
    : null;

  const handleCreateShareLink = useCallback(() => {
    if (!conversation) return;
    shareConversation.mutate(conversation.id);
  }, [conversation, shareConversation]);

  const handleRemoveShareLink = useCallback(() => {
    if (!conversation) return;
    unshareConversation.mutate(conversation.id);
  }, [conversation, unshareConversation]);

  const handleCopyLink = useCallback(async () => {
    if (!shareUrl) return;

    try {
      await navigator.clipboard.writeText(shareUrl);
      setCopied(true);
      toast.success(t("query.share.copied", "Link copied to clipboard"));
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error("Failed to copy:", error);
      toast.error(t("query.share.copyError", "Failed to copy link"));
    }
  }, [shareUrl, t]);

  const handleOpenLink = useCallback(() => {
    if (!shareUrl) return;
    window.open(shareUrl, "_blank", "noopener,noreferrer");
  }, [shareUrl]);

  if (!conversation) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Share2 className="h-5 w-5" />
            {t("query.share.title", "Share Conversation")}
          </DialogTitle>
          <DialogDescription>
            {shareUrl
              ? t(
                  "query.share.descriptionActive",
                  "Anyone with this link can view this conversation."
                )
              : t(
                  "query.share.description",
                  "Create a shareable link to give others read-only access to this conversation."
                )}
          </DialogDescription>
        </DialogHeader>

        <div className="py-4 space-y-4">
          {/* Conversation Info */}
          <div className="rounded-lg border bg-muted/30 p-3 space-y-1">
            <p className="font-medium text-sm truncate">{conversation.title}</p>
            <p className="text-xs text-muted-foreground">
              {conversation.messages.length}{" "}
              {t("query.messages", "messages")} ·{" "}
              {t("query.mode", "Mode")}: {conversation.mode}
            </p>
          </div>

          {/* Share Link Section */}
          {shareUrl ? (
            <div className="space-y-3">
              <Label className="text-xs font-medium">
                {t("query.share.linkLabel", "Share Link")}
              </Label>
              <div className="flex gap-2">
                <Input
                  value={shareUrl}
                  readOnly
                  className="h-9 text-sm bg-muted/30 pr-2"
                />
                <Button
                  variant="outline"
                  size="icon"
                  className="h-9 w-9 shrink-0"
                  onClick={handleCopyLink}
                  aria-label={t("common.copy", "Copy")}
                >
                  {copied ? (
                    <Check className="h-4 w-4 text-green-500" />
                  ) : (
                    <Copy className="h-4 w-4" />
                  )}
                </Button>
                <Button
                  variant="outline"
                  size="icon"
                  className="h-9 w-9 shrink-0"
                  onClick={handleOpenLink}
                  aria-label={t("common.open", "Open")}
                >
                  <ExternalLink className="h-4 w-4" />
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                {t(
                  "query.share.warning",
                  "⚠️ Anyone with this link can view all messages in this conversation."
                )}
              </p>
            </div>
          ) : (
            <div className="flex flex-col items-center py-6 space-y-4">
              <div className="w-16 h-16 rounded-full bg-muted/50 flex items-center justify-center">
                <Link className="h-8 w-8 text-muted-foreground" />
              </div>
              <p className="text-sm text-muted-foreground text-center max-w-xs">
                {t(
                  "query.share.noLinkYet",
                  "This conversation is not shared yet. Create a link to share it with others."
                )}
              </p>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="flex justify-between">
          {shareUrl ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleRemoveShareLink}
              disabled={unshareConversation.isPending}
              className="text-destructive hover:text-destructive"
            >
              {unshareConversation.isPending ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <Trash2 className="h-4 w-4 mr-2" />
              )}
              {t("query.share.removeLink", "Remove Link")}
            </Button>
          ) : (
            <div /> // Spacer
          )}
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              {t("common.close", "Close")}
            </Button>
            {!shareUrl && (
              <Button
                onClick={handleCreateShareLink}
                disabled={shareConversation.isPending}
              >
                {shareConversation.isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    {t("query.share.creating", "Creating...")}
                  </>
                ) : (
                  <>
                    <Link className="h-4 w-4 mr-2" />
                    {t("query.share.createLink", "Create Link")}
                  </>
                )}
              </Button>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default ShareDialog;
