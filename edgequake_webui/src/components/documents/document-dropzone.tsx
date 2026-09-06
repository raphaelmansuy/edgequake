'use client';

import { cn } from '@/lib/utils';
import { Upload } from 'lucide-react';
import type React from 'react';
import type { DropzoneInputProps, DropzoneRootProps } from 'react-dropzone';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  shouldShowVisionExtractControls,
  VisionSettingsPanel,
  type VisionExtractDraft,
} from '@/components/settings/vision-extract-controls';
import { SecurityFieldsForm } from '@/components/security/security-fields-form';
import type { SecurityFields } from '@/lib/security/security-fields';
import { useLlmModels } from '@/hooks/use-providers';
import { useTranslation } from 'react-i18next';
import { MAX_UPLOAD_LABEL } from '@/lib/api/upload-limits';
import { formatWorkspaceDefaultPdfParserLabel } from '@/lib/pdf/resolve-pdf-parser-backend';
import {
  effectiveEffortWhenAuto,
  modelSupportsThinking,
  supportedReasoningEffortsForModel,
} from '@/lib/settings/reasoning-effort-supported';
import { CONTAIN_COMBO, CONTAIN_ROW, CONTAIN_STACK, CONTAIN_TRIGGER } from '@/lib/ui/containment';
import type { PdfParserBackend } from '@/types/graph';
import { useMemo } from 'react';

/**
 * Props for the DocumentDropzone component.
 */
export interface DocumentDropzoneProps {
  /** Props to spread on the dropzone container */
  getRootProps: <T extends DropzoneRootProps>(props?: T) => T;
  /** Props to spread on the hidden file input */
  getInputProps: <T extends DropzoneInputProps>(props?: T) => T;
  /** Whether a drag operation is currently active over the zone */
  isDragActive: boolean;
  /** Function to programmatically open file dialog (explicit click handler) */
  openFileDialog: () => void;
  /** Per-upload PDF parser backend override. */
  pdfParserBackend: 'default' | 'vision' | 'edgeparse' | 'auto';
  /** Change handler for the PDF parser override selector. */
  onPdfParserBackendChange: (
    value: 'default' | 'vision' | 'edgeparse' | 'auto',
  ) => void;
  /**
   * Workspace default `pdf_parser_backend` — shown in the inherit option label
   * (e.g. Workspace Default (Vision)). Falls back to server → Vision when unset.
   */
  workspacePdfParserBackend?: PdfParserBackend | null;
  /** SPEC-109: optional vision reasoning effort for VLM convert. */
  visionReasoningEffort?: string;
  onVisionReasoningEffortChange?: (value: string | undefined) => void;
  /** SPEC-015V */
  visionExtract?: VisionExtractDraft;
  onVisionExtractChange?: (value: VisionExtractDraft) => void;
  /** SPEC-113: vision model identity for thinking capability honesty. */
  visionProvider?: string | null;
  visionModel?: string | null;
  /**
   * SPEC-048: compact chrome while ingestion is working so progress UI stays primary.
   */
  quiet?: boolean;
  /**
   * SPEC-099 LAW-099-4: denser band when feedback zone has live work.
   * Always remains a full-width drop target (never removed).
   */
  collapsed?: boolean;
  /** SPEC-146 upload security labels. */
  securityFields?: SecurityFields;
  onSecurityFieldsChange?: (value: SecurityFields) => void;
  /** When true, show SecurityFieldsForm (typically when DOC_ABAC is on). */
  showSecurityFields?: boolean;
  /** Bubbles SecurityFields expand state for documents-chrome max-height. */
  onSecurityExpandedChange?: (open: boolean) => void;
}

function ParserSelect({
  pdfParserBackend,
  onPdfParserBackendChange,
  workspacePdfParserBackend,
  compact,
  /** When true, omit the side label — used inside the Vision combo row. */
  hideSideLabel,
  triggerClassName,
}: {
  pdfParserBackend: 'default' | 'vision' | 'edgeparse' | 'auto';
  onPdfParserBackendChange: (
    value: 'default' | 'vision' | 'edgeparse' | 'auto',
  ) => void;
  workspacePdfParserBackend?: PdfParserBackend | null;
  compact: boolean;
  hideSideLabel?: boolean;
  triggerClassName?: string;
}) {
  const { t } = useTranslation();
  const workspaceDefaultLabel = formatWorkspaceDefaultPdfParserLabel(
    t,
    workspacePdfParserBackend,
  );
  return (
    <div
      className={cn(
        hideSideLabel ? CONTAIN_ROW : CONTAIN_STACK,
        hideSideLabel && 'flex-1',
        compact && 'opacity-80',
      )}
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => event.stopPropagation()}
    >
      {!compact && !hideSideLabel && (
        <span className="text-xs text-muted-foreground shrink-0 lg:whitespace-nowrap">
          {t('documents.upload.pdfParser', 'Parser for this upload')}
        </span>
      )}
      <div className={cn(CONTAIN_TRIGGER, 'flex-1')}>
        <Select
          value={pdfParserBackend}
          onValueChange={(value: 'default' | 'vision' | 'edgeparse' | 'auto') =>
            onPdfParserBackendChange(value)
          }
        >
          <SelectTrigger
            className={cn(
              'bg-background w-full',
              CONTAIN_TRIGGER,
              triggerClassName ??
                (compact ? 'h-7 text-xs' : 'h-9'),
            )}
            data-testid="spec038-upload-parser-select"
            title={
              pdfParserBackend === 'default' ? workspaceDefaultLabel : undefined
            }
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="default">{workspaceDefaultLabel}</SelectItem>
            <SelectItem value="vision">
              {t('documents.upload.pdfParserVision', 'Vision')}
            </SelectItem>
            <SelectItem value="edgeparse">
              {t('documents.upload.pdfParserEdgeParse', 'EdgeParse')}
            </SelectItem>
            <SelectItem value="auto">
              {t('documents.upload.pdfParserAuto', 'Auto')}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}

/**
 * Always-on file upload drop zone (idle expand / busy collapse).
 *
 * SPEC-099: the drop zone is never removed — collapse only shrinks chrome.
 * Drag-and-drop, click, and keyboard activation remain available.
 *
 * @implements FEAT0001 - Document ingestion with entity extraction
 * @implements SPEC-099 F-099-04 - collapse when feedback zone has live work
 */
export function DocumentDropzone({
  getRootProps,
  getInputProps,
  isDragActive,
  openFileDialog,
  pdfParserBackend,
  onPdfParserBackendChange,
  workspacePdfParserBackend,
  visionReasoningEffort,
  onVisionReasoningEffortChange,
  visionExtract,
  onVisionExtractChange,
  visionProvider,
  visionModel,
  quiet = false,
  collapsed = false,
  securityFields,
  onSecurityFieldsChange,
  showSecurityFields = false,
  onSecurityExpandedChange,
}: DocumentDropzoneProps) {
  const { t } = useTranslation();
  const { data: llmCatalog } = useLlmModels();
  const visionThinkingSupported = useMemo(
    () => modelSupportsThinking(llmCatalog?.models, visionProvider, visionModel),
    [llmCatalog?.models, visionProvider, visionModel],
  );
  const visionEffortSupported = useMemo(
    () =>
      supportedReasoningEffortsForModel(
        llmCatalog?.models,
        visionProvider,
        visionModel,
      ),
    [llmCatalog?.models, visionProvider, visionModel],
  );
  const compact = quiet || collapsed;
  const workspaceIsVision =
    !workspacePdfParserBackend ||
    workspacePdfParserBackend === 'vision' ||
    workspacePdfParserBackend === 'auto';
  const showVisionPanel =
    !compact &&
    !collapsed &&
    typeof onVisionExtractChange === 'function' &&
    visionExtract &&
    shouldShowVisionExtractControls(pdfParserBackend, workspaceIsVision);
  const showVisionEffort =
    showVisionPanel && typeof onVisionReasoningEffortChange === 'function';

  const rootProps = getRootProps({
    onClick: (e: React.MouseEvent) => {
      e.stopPropagation();
      openFileDialog();
    },
    onKeyDown: (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        e.stopPropagation();
        openFileDialog();
      }
    },
    role: 'button' as const,
    'aria-label': collapsed
      ? t('documents.upload.uploadCollapsed', 'Add files — click or drop')
      : t('documents.upload.uploadDrop', 'Upload files by clicking or dragging'),
    tabIndex: 0,
  });

  return (
    <div
      {...rootProps}
      data-testid="document-dropzone"
      data-upload="true"
      data-quiet={quiet ? 'true' : 'false'}
      data-collapsed={collapsed ? 'true' : 'false'}
      className={cn(
        // Always a full-width single-line drop band — never a multi-paragraph hero
        // that steals the inventory flex budget (SPEC-099 scroll layout).
        'w-full border-dashed cursor-pointer transition-all duration-200',
        'flex flex-wrap items-center gap-3 min-w-0',
        collapsed
          ? 'rounded-md border px-3 py-1.5 gap-2'
          : compact
            ? 'rounded-lg border px-3 py-2 gap-2'
            : 'rounded-lg border-2 px-4 py-2.5 gap-3',
        isDragActive
          ? 'border-primary bg-primary/5 ring-2 ring-primary/20'
          : collapsed || quiet
            ? 'border-muted-foreground/20 bg-muted/15 hover:border-primary/40 hover:bg-muted/25'
            : 'border-muted-foreground/20 hover:border-primary/50 hover:bg-muted/30',
      )}
    >
      <input {...getInputProps()} data-testid="document-dropzone-input" />
      <div
        className={cn(
          'rounded-lg transition-all shrink-0',
          compact || collapsed ? 'p-1.5' : 'p-2',
          isDragActive ? 'bg-primary/10' : 'bg-muted/50',
        )}
      >
        <Upload
          className={cn(
            'transition-all duration-200',
            compact || collapsed ? 'h-4 w-4' : 'h-5 w-5',
            isDragActive ? 'text-primary scale-110' : 'text-muted-foreground',
          )}
        />
      </div>
      <div className="min-w-0 flex-1 overflow-hidden">
        {isDragActive ? (
          <p className="truncate text-sm font-medium text-primary">
            {t('documents.upload.uploadDropActive', 'Drop files here')}
          </p>
        ) : collapsed ? (
          <p className="truncate text-xs text-muted-foreground">
            {t('documents.upload.addFilesDrop', 'Drop files here or click to add')}
          </p>
        ) : quiet ? (
          <p className="truncate text-xs text-muted-foreground">
            {t(
              'documents.upload.uploadWhileWorking',
              'Add more files anytime · max {{limit}}',
              { limit: MAX_UPLOAD_LABEL },
            )}
          </p>
        ) : (
          <p
            className="truncate text-sm text-muted-foreground"
            title={t(
              'documents.upload.uploadDropWithLimit',
              'Drag & drop or click to upload • TXT, MD, JSON, PDF, PNG, JPG, GIF, WEBP (max {{limit}}) · DOCX/Excel not supported',
              { limit: MAX_UPLOAD_LABEL },
            )}
          >
            {t(
              'documents.upload.uploadDropWithLimit',
              'Drag & drop or click to upload • TXT, MD, JSON, PDF, PNG, JPG, GIF, WEBP (max {{limit}}) · DOCX/Excel not supported',
              { limit: MAX_UPLOAD_LABEL },
            )}
          </p>
        )}
      </div>
      <div
        className={cn(
          CONTAIN_COMBO,
          (compact || collapsed) && 'opacity-80',
        )}
        data-testid="upload-parser-vision-combo"
        onClick={(event) => event.stopPropagation()}
        onKeyDown={(event) => event.stopPropagation()}
      >
        <ParserSelect
          pdfParserBackend={pdfParserBackend}
          onPdfParserBackendChange={onPdfParserBackendChange}
          workspacePdfParserBackend={workspacePdfParserBackend}
          compact={compact || collapsed}
          hideSideLabel={false}
          triggerClassName={
            compact || collapsed ? 'h-7 text-xs' : 'h-9'
          }
        />
        {showVisionPanel && visionExtract && onVisionExtractChange ? (
          <VisionSettingsPanel
            value={visionExtract}
            onChange={onVisionExtractChange}
            showInheritHint={pdfParserBackend === 'default'}
            compact={compact || collapsed}
            className="w-full lg:w-auto justify-self-stretch lg:justify-self-end"
            effort={
              showVisionEffort && onVisionReasoningEffortChange
                ? {
                    value: visionReasoningEffort,
                    onChange: onVisionReasoningEffortChange,
                    supported: visionEffortSupported,
                    thinkingSupported: visionThinkingSupported,
                    effectiveWhenAuto: effectiveEffortWhenAuto(
                      llmCatalog?.models,
                      visionProvider,
                      visionModel,
                      'structured',
                    ),
                  }
                : undefined
            }
          />
        ) : null}
      </div>
      {/* SPEC-146: keep SecurityFields visible even when the upload slot is
          collapsed by the feedback zone — labels must stay on the dropzone. */}
      {showSecurityFields && securityFields && onSecurityFieldsChange ? (
        <div
          className="w-full basis-full mt-2 pt-2 border-t border-dashed border-muted-foreground/20"
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => e.stopPropagation()}
        >
          <SecurityFieldsForm
            value={securityFields}
            onChange={onSecurityFieldsChange}
            compact
            onExpandedChange={onSecurityExpandedChange}
          />
        </div>
      ) : null}
    </div>
  );
}
