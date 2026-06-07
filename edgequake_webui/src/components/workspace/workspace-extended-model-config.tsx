"use client";

import { ProviderIcon } from "@/components/providers/provider-icon";
import {
  PdfParserBackendField,
  type PdfParserBackendChoice,
} from "@/components/settings/pdf-parser-backend-field";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  LLMModelSelector,
  type LLMSelection,
} from "@/components/workspace/llm-model-selector";
import type { Workspace } from "@/types";
import { AlertTriangle, Eye, Gauge, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface WorkspaceExtendedModelConfigProps {
  workspace: Workspace;
  isEditing: boolean;
  selectedVisionLLM: LLMSelection | undefined;
  selectedPdfParserBackend: PdfParserBackendChoice;
  onVisionLlmChange: (value: LLMSelection | undefined) => void;
  onPdfParserBackendChange: (value: PdfParserBackendChoice) => void;
  visionLLMChanged: boolean;
}

/** Vision LLM + PDF parser cards (dashboard workspace route only, SPEC-040). */
export function WorkspaceExtendedModelConfig({
  workspace,
  isEditing,
  selectedVisionLLM,
  selectedPdfParserBackend,
  onVisionLlmChange,
  onPdfParserBackendChange,
  visionLLMChanged,
}: WorkspaceExtendedModelConfigProps) {
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-orange-600" />
            {t("workspace.visionLlmConfig", "Vision LLM (PDF Extraction)")}
          </CardTitle>
          <CardDescription>
            {t(
              "workspace.visionLlmConfigDesc",
              "Multimodal model used for PDF page rendering and text extraction. Overrides server default for this workspace.",
            )}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isEditing ? (
            <>
              <LLMModelSelector
                value={selectedVisionLLM}
                onChange={onVisionLlmChange}
                showUsageHint
              />
              {visionLLMChanged && (
                <div className="flex items-center gap-2 p-3 bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded-lg">
                  <AlertTriangle className="h-4 w-4 text-orange-600" />
                  <span className="text-sm text-orange-700 dark:text-orange-300">
                    {t(
                      "workspace.visionLlmChangeWarning",
                      "New Vision LLM will be used for all subsequent PDF uploads.",
                    )}
                  </span>
                </div>
              )}
            </>
          ) : (
            <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
              <ProviderIcon providerId={workspace.vision_llm_provider} />
              <div>
                <div className="font-medium">
                  {workspace.vision_llm_model ||
                    t("workspace.serverDefault", "Server Default")}
                </div>
                <div className="text-sm text-muted-foreground capitalize">
                  {workspace.vision_llm_provider ||
                    t("workspace.autoDetect", "Auto-detected")}
                </div>
              </div>
              {workspace.vision_llm_provider && workspace.vision_llm_model && (
                <Badge variant="outline" className="ml-auto">
                  {`${workspace.vision_llm_provider}/${workspace.vision_llm_model}`}
                </Badge>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            {selectedPdfParserBackend === "vision" ? (
              <Eye className="h-5 w-5 text-amber-600" />
            ) : (
              <Gauge className="h-5 w-5 text-amber-600" />
            )}
            {t("workspace.pdfParserConfig", "PDF Parser")}
          </CardTitle>
          <CardDescription>
            {t(
              "workspace.pdfParserConfigDesc",
              "Choose the default parser for new PDF uploads in this workspace. EdgeParse is best for digital PDFs; Vision is better for scanned or image-heavy files.",
            )}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <PdfParserBackendField
            value={selectedPdfParserBackend}
            isEditing={isEditing}
            onChange={onPdfParserBackendChange}
          />
          {isEditing && (
            <div className="flex items-center gap-2 p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
              <AlertTriangle className="h-4 w-4 text-amber-600" />
              <span className="text-sm text-amber-700 dark:text-amber-300">
                {t(
                  "workspace.pdfParserChangeWarning",
                  "This default applies to subsequent PDF uploads. Existing documents keep their original extraction method unless reprocessed.",
                )}
              </span>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
