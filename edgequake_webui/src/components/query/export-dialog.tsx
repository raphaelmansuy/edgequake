"use client";

/**
 * Export Dialog Component
 *
 * Provides a dialog for exporting conversations to various formats.
 */

import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { downloadAsJSON, downloadAsMarkdown } from "@/lib/export-conversation";
import type { ConversationWithMessages } from "@/types";
import { Download, FileJson, FileText, Loader2 } from "lucide-react";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

// ============================================================================
// Types
// ============================================================================

type ExportFormat = "markdown" | "json";

interface ExportDialogProps {
  conversation: ConversationWithMessages | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

// ============================================================================
// Component
// ============================================================================

export function ExportDialog({
  conversation,
  open,
  onOpenChange,
}: ExportDialogProps) {
  const { t } = useTranslation();
  const [format, setFormat] = useState<ExportFormat>("markdown");
  const [isExporting, setIsExporting] = useState(false);

  const handleExport = useCallback(async () => {
    if (!conversation) return;

    setIsExporting(true);
    try {
      // Simulate async operation for UX consistency
      await new Promise((resolve) => setTimeout(resolve, 300));

      if (format === "markdown") {
        downloadAsMarkdown(conversation);
        toast.success(t("query.export.successMarkdown", "Exported as Markdown"));
      } else {
        downloadAsJSON(conversation);
        toast.success(t("query.export.successJSON", "Exported as JSON"));
      }

      onOpenChange(false);
    } catch (error) {
      console.error("Export error:", error);
      toast.error(t("query.export.error", "Failed to export conversation"));
    } finally {
      setIsExporting(false);
    }
  }, [conversation, format, onOpenChange, t]);

  if (!conversation) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Download className="h-5 w-5" />
            {t("query.export.title", "Export Conversation")}
          </DialogTitle>
          <DialogDescription>
            {t(
              "query.export.description",
              "Choose a format to export your conversation."
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

          {/* Format Selection */}
          <RadioGroup
            value={format}
            onValueChange={(v) => setFormat(v as ExportFormat)}
            className="grid grid-cols-2 gap-3"
          >
            <Label
              htmlFor="format-markdown"
              className={`flex flex-col items-center gap-2 rounded-lg border p-4 cursor-pointer transition-colors hover:bg-muted/50 ${
                format === "markdown"
                  ? "border-primary bg-primary/5"
                  : "border-muted"
              }`}
            >
              <RadioGroupItem
                value="markdown"
                id="format-markdown"
                className="sr-only"
              />
              <FileText
                className={`h-8 w-8 ${
                  format === "markdown"
                    ? "text-primary"
                    : "text-muted-foreground"
                }`}
              />
              <div className="text-center">
                <p className="font-medium text-sm">Markdown</p>
                <p className="text-xs text-muted-foreground">.md</p>
              </div>
            </Label>

            <Label
              htmlFor="format-json"
              className={`flex flex-col items-center gap-2 rounded-lg border p-4 cursor-pointer transition-colors hover:bg-muted/50 ${
                format === "json" ? "border-primary bg-primary/5" : "border-muted"
              }`}
            >
              <RadioGroupItem
                value="json"
                id="format-json"
                className="sr-only"
              />
              <FileJson
                className={`h-8 w-8 ${
                  format === "json" ? "text-primary" : "text-muted-foreground"
                }`}
              />
              <div className="text-center">
                <p className="font-medium text-sm">JSON</p>
                <p className="text-xs text-muted-foreground">.json</p>
              </div>
            </Label>
          </RadioGroup>

          {/* Format Description */}
          <p className="text-xs text-muted-foreground">
            {format === "markdown"
              ? t(
                  "query.export.markdownDescription",
                  "Human-readable format, ideal for documentation and sharing."
                )
              : t(
                  "query.export.jsonDescription",
                  "Machine-readable format, can be re-imported later."
                )}
          </p>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel", "Cancel")}
          </Button>
          <Button onClick={handleExport} disabled={isExporting}>
            {isExporting ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                {t("query.export.exporting", "Exporting...")}
              </>
            ) : (
              <>
                <Download className="h-4 w-4 mr-2" />
                {t("query.export.button", "Export")}
              </>
            )}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default ExportDialog;
