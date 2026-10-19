'use client';

import { cn } from '@/lib/utils';
import { SlidersHorizontal, Upload } from 'lucide-react';
import type React from 'react';
import { useId, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import type { DropzoneInputProps, DropzoneRootProps } from 'react-dropzone';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { UploadExtractionModeSelect } from '@/components/documents/upload-extraction-mode-select';
import type {
  ExtractionModeWord,
  UploadExtractionChoice,
} from '@/constants/extraction-mode';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
} from '@/components/ui/select';
import {
  shouldShowVisionExtractControls,
  VisionSettingsPanel,
  type VisionExtractDraft,
} from '@/components/settings/vision-extract-controls';
import { useLlmModels } from '@/hooks/use-providers';
import { useTranslation } from 'react-i18next';
import { MAX_UPLOAD_LABEL } from '@/lib/api/upload-limits';
import { formatWorkspaceDefaultPdfParserLabel, formatWorkspaceDefaultPdfParserShortLabel } from '@/lib/pdf/resolve-pdf-parser-backend';
import {
  effectiveEffortWhenAuto,
  modelSupportsThinking,
  supportedReasoningEffortsForModel,
} from '@/lib/settings/reasoning-effort-supported';
import type { UploadPdfParserChoice } from '@/lib/pdf/resolve-pdf-parser-backend';
import type { PdfParserBackend } from '@/types/graph';
import {
  DROPZONE_ROOMY_MIN_HEIGHT_PX,
  resolveDropzoneFillLayout,
  type DropzoneFillLayout,
} from '@/lib/documents/dropzone-fill-layout';

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
  pdfParserBackend: UploadPdfParserChoice;
  /** Change handler for the PDF parser override selector. */
  onPdfParserBackendChange: (value: UploadPdfParserChoice) => void;
  /**
   * Workspace default `pdf_parser_backend` — shown in the inherit option label
   * (e.g. Workspace Default (Vision)). Falls back to server → Vision when unset.
   */
  workspacePdfParserBackend?: PdfParserBackend | null;
  /** SPEC-160: per-upload extraction choice. Omit the handler to hide the select. */
  extractionMode?: UploadExtractionChoice;
  onExtractionModeChange?: (value: UploadExtractionChoice) => void;
  /** SPEC-160: mode a default upload would get, and the workspace's decision model. */
  workspaceExtractionMode?: ExtractionModeWord | null;
  workspaceDecisionModel?: string | null;
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
  /**
   * Fill the parent panel (docking Upload zone). Stretches to 100% width/height
   * with content centered inside the dashed frame.
   */
  fill?: boolean;
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
  pdfParserBackend: UploadPdfParserChoice;
  onPdfParserBackendChange: (value: UploadPdfParserChoice) => void;
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
  const workspaceDefaultTrigger = compact
    ? formatWorkspaceDefaultPdfParserShortLabel(t, workspacePdfParserBackend)
    : workspaceDefaultLabel;
  const parserWords: Record<Exclude<UploadPdfParserChoice, 'default'>, string> = {
    vision: t('documents.upload.pdfParserVision', 'Vision'),
    edgeparse: t('documents.upload.pdfParserEdgeParse', 'EdgeParse'),
    'edgeparse-ocr': t('documents.upload.pdfParserEdgeParseOcr', 'EdgeParse + OCR'),
    auto: t('documents.upload.pdfParserAuto', 'Auto'),
  };
  const triggerText =
    pdfParserBackend === 'default' ? workspaceDefaultTrigger : parserWords[pdfParserBackend];
  return (
    <div
      className={cn(
        'flex min-w-0 items-center gap-2',
        hideSideLabel ? 'min-w-0 flex-1' : 'shrink-0',
      )}
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => event.stopPropagation()}
    >
      {/* Always labelled so the select reads as "applies to the next upload",
          not a free-floating control (compact density keeps the short form). */}
      {!hideSideLabel && (
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {compact
            ? t('documents.upload.pdfParserShort', 'Parser')
            : t('documents.upload.pdfParser', 'Parser for this upload')}
        </span>
      )}
      <Select
        value={pdfParserBackend}
        onValueChange={(value: UploadPdfParserChoice) =>
          onPdfParserBackendChange(value)
        }
      >
        <SelectTrigger
          className={cn(
            'min-w-0 bg-background',
            triggerClassName ??
              (compact
                ? 'h-7 w-auto min-w-0 max-w-[14rem] text-xs'
                : 'h-9 w-auto min-w-0 max-w-[18rem]'),
          )}
          data-testid="spec038-upload-parser-select"
          title={
            pdfParserBackend === 'default' ? workspaceDefaultLabel : triggerText
          }
        >
          <span className="min-w-0 flex-1 truncate text-left">{triggerText}</span>
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="default">{workspaceDefaultLabel}</SelectItem>
          <SelectItem value="vision">
            {t('documents.upload.pdfParserVision', 'Vision')}
          </SelectItem>
          <SelectItem value="edgeparse">
            {t('documents.upload.pdfParserEdgeParse', 'EdgeParse')}
          </SelectItem>
          <SelectItem value="edgeparse-ocr">
            {t('documents.upload.pdfParserEdgeParseOcr', 'EdgeParse + OCR')}
          </SelectItem>
          <SelectItem value="auto">
            {t('documents.upload.pdfParserAuto', 'Auto')}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}

const FORMAT_CODES = ['TXT', 'MD', 'JSON', 'PDF'] as const;

function ChooseFilesButton({
  label,
  onChoose,
  className,
}: {
  label: string;
  onChoose: () => void;
  className?: string;
}) {
  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      className={cn('bg-background shadow-xs', className)}
      data-testid="document-dropzone-browse"
      onClick={(event) => {
        event.stopPropagation();
        onChoose();
      }}
      onKeyDown={(event) => event.stopPropagation()}
    >
      {label}
    </Button>
  );
}

function FormatChips({ label, imagesLabel }: { label: string; imagesLabel: string }) {
  return (
    <ul
      className="mt-4 flex max-w-md flex-wrap items-center justify-center gap-1.5"
      aria-label={label}
      data-testid="document-dropzone-formats"
    >
      {[...FORMAT_CODES, imagesLabel].map((chip) => (
        <li
          key={chip}
          className="rounded-md border border-border/80 bg-background px-1.5 py-0.5 text-[11px] font-medium tracking-wide text-muted-foreground"
        >
          {chip}
        </li>
      ))}
    </ul>
  );
}

/**
 * Width policy shared by every select in the settings bar:
 * stack fills its column, row and free shrink and wrap instead of overlapping.
 */
function settingTriggerWidth(
  layout: 'stack' | 'row' | 'free',
  /** A literal Tailwind class (e.g. `max-w-[16rem]`) so the JIT scanner sees it. */
  maxWidthClass: string,
): string {
  if (layout === 'stack') return 'w-full max-w-none min-w-0';
  return cn('min-w-0 w-full flex-1 basis-[9rem]', maxWidthClass);
}

function settingControlShell(stacked: boolean): string {
  return stacked ? 'w-full min-w-0' : 'min-w-0 flex-1 basis-[9rem] max-w-full';
}

function DropzoneSettingsBar({
  stacked,
  label,
  hint,
  collapseWhenNarrow,
  settingsLabel,
  children,
}: {
  stacked: boolean;
  label: string;
  hint: string;
  /** Narrow, short bands tuck the controls behind one button instead of clipping. */
  collapseWhenNarrow: boolean;
  settingsLabel: string;
  children: ReactNode;
}) {
  const stop = {
    onClick: (event: React.MouseEvent) => event.stopPropagation(),
    onKeyDown: (event: React.KeyboardEvent) => event.stopPropagation(),
  };
  return (
    <>
      {collapseWhenNarrow ? (
        <div
          className={'flex shrink-0 border-t border-border/70 bg-background px-3 py-1.5 @[30rem]/dz:hidden'}
          data-testid="upload-settings-popover-bar"
          {...stop}
        >
          <Popover>
            <PopoverTrigger asChild>
              <Button type="button" variant="ghost" size="sm" className="h-7 gap-1.5 px-2 text-xs" title={hint}>
                <SlidersHorizontal className="h-3.5 w-3.5" aria-hidden />
                {settingsLabel}
              </Button>
            </PopoverTrigger>
            <PopoverContent align="start" className="flex w-72 flex-col gap-2 p-3" {...stop}>
              <span className="text-xs font-medium text-muted-foreground">{label}</span>
              {children}
            </PopoverContent>
          </Popover>
        </div>
      ) : null}
      <div
        className={cn(
          'flex shrink-0 cursor-auto gap-2 overflow-hidden border-t border-border/70 bg-background px-3 py-2',
          stacked ? 'flex-col items-stretch' : 'flex-row flex-wrap items-center',
          collapseWhenNarrow && '@max-[30rem]/dz:hidden',
        )}
        data-testid="upload-parser-vision-combo"
        {...stop}
      >
        <span className="shrink-0 text-xs font-medium text-muted-foreground" title={hint}>
          {label}
        </span>
        <div
          className={cn(
            'flex min-w-0 items-stretch gap-2',
            stacked ? 'w-full flex-col' : 'flex-1 flex-wrap',
          )}
        >
          {children}
        </div>
      </div>
    </>
  );
}

/**
 * Always-on file upload drop zone (idle expand / busy collapse).
 *
 * SPEC-099: the drop zone is never removed — collapse only shrinks chrome.
 * Drag-and-drop, click, and keyboard activation remain available.
 *
 * Tall panels center one invitation and dock PDF parser settings on the
 * bottom edge, so the dashed area stays a single drop target.
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
  extractionMode = 'default',
  onExtractionModeChange,
  workspaceExtractionMode,
  workspaceDecisionModel,
  visionReasoningEffort,
  onVisionReasoningEffortChange,
  visionExtract,
  onVisionExtractChange,
  visionProvider,
  visionModel,
  quiet = false,
  collapsed = false,
  fill = false,
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

  const hintId = useId();
  const rootRef = useRef<HTMLDivElement | null>(null);
  const [fillLayout, setFillLayout] = useState<DropzoneFillLayout>('hero');
  const [measured, setMeasured] = useState(false);
  const [boxHeight, setBoxHeight] = useState(0);

  useLayoutEffect(() => {
    if (!fill) return;
    const el = rootRef.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const apply = () => {
      const { width, height } = el.getBoundingClientRect();
      setBoxHeight(height);
      setFillLayout(resolveDropzoneFillLayout(width, height));
      setMeasured(true);
    };
    apply();
    const ro = new ResizeObserver(apply);
    ro.observe(el);
    return () => ro.disconnect();
  }, [fill]);

  const fillIsRow = fill && fillLayout === 'row';
  const fillIsStack = fill && fillLayout === 'stack';
  const fillIsHero = fill && fillLayout === 'hero';
  const roomy =
    !collapsed &&
    fillIsHero &&
    measured &&
    boxHeight >= DROPZONE_ROOMY_MIN_HEIGHT_PX;
  const composed = Boolean(fill) && !fillIsRow;
  const imagesLabel = t('documents.upload.formatImages', 'Images');
  const formatLine = `TXT, MD, JSON, PDF, ${imagesLabel}`;
  const limitHint = t(
    'documents.upload.formatsLimit',
    'Up to {{limit}} · Word and Excel aren’t accepted',
    { limit: MAX_UPLOAD_LABEL },
  );

  const invitationTitle = isDragActive
    ? t('documents.upload.uploadDropRelease', 'Release to upload')
    : quiet
      ? t('documents.upload.addMoreFiles', 'Add more files')
      : t('documents.upload.addDocuments', 'Add documents');
  const invitationHint = isDragActive
    ? formatLine
    : quiet
      ? t(
          'documents.upload.addMoreFilesHint',
          'You can drop more files while processing continues.',
        )
      : t(
          'documents.upload.addDocumentsHint',
          'Drop files here, or choose them from your computer.',
        );

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
    // WHY group, not button: the root hosts nested selects (axe nested-interactive).
    role: 'group' as const,
    'aria-label': invitationTitle,
    'aria-describedby': hintId,
    tabIndex: 0,
  });

  const { ref: dropRef, ...dropRootProps } = rootProps as typeof rootProps & {
    ref?: React.Ref<HTMLDivElement>;
  };
  const setRefs = (node: HTMLDivElement | null) => {
    rootRef.current = node;
    if (typeof dropRef === 'function') dropRef(node);
    else if (dropRef && typeof dropRef === 'object') {
      (dropRef as React.MutableRefObject<HTMLDivElement | null>).current = node;
    }
  };

  const compactHint = isDragActive
    ? formatLine
    : quiet
      ? invitationHint
      : t(
          'documents.upload.dropOrChooseLimit',
          'Drop files or choose them · up to {{limit}}',
          { limit: MAX_UPLOAD_LABEL },
        );

  const fillLayoutKind = fillIsStack ? 'stack' : fillIsRow ? 'row' : 'free';
  const controlShell = settingControlShell(fillIsStack);
  const settings = (
    <>
      <div className={controlShell}>
        <ParserSelect
          pdfParserBackend={pdfParserBackend}
          onPdfParserBackendChange={onPdfParserBackendChange}
          workspacePdfParserBackend={workspacePdfParserBackend}
          compact={compact || collapsed || Boolean(fill)}
          hideSideLabel={collapsed || Boolean(fill)}
          triggerClassName={
            compact || collapsed || fill
              ? cn('h-7 text-xs', settingTriggerWidth(fillLayoutKind, 'max-w-[16rem]'))
              : 'h-9 w-auto min-w-0 max-w-[18rem]'
          }
        />
      </div>
      {onExtractionModeChange ? (
        <div className={controlShell}>
          <UploadExtractionModeSelect
            value={extractionMode}
            onValueChange={onExtractionModeChange}
            workspaceMode={workspaceExtractionMode}
            decisionModel={workspaceDecisionModel}
            compact={compact || collapsed || Boolean(fill)}
            hideSideLabel={collapsed || Boolean(fill)}
            singleLineWarning={fillIsRow}
            triggerClassName={
              fill || compact || collapsed
                ? settingTriggerWidth(fillLayoutKind, 'max-w-[16rem]')
                : undefined
            }
          />
        </div>
      ) : null}
      {showVisionPanel && visionExtract && onVisionExtractChange ? (
        <VisionSettingsPanel
          value={visionExtract}
          onChange={onVisionExtractChange}
          showInheritHint={pdfParserBackend === 'default'}
          compact={compact || collapsed || Boolean(fill)}
          className={fillIsStack ? 'w-full min-w-0' : cn(controlShell, 'min-w-0')}
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
    </>
  );

  return (
    <div
      {...dropRootProps}
      ref={setRefs}
      data-testid="document-dropzone"
      data-upload="true"
      data-quiet={quiet ? 'true' : 'false'}
      data-collapsed={collapsed ? 'true' : 'false'}
      data-fill={fill ? 'true' : 'false'}
      data-fill-layout={fill ? fillLayout : undefined}
      data-density={!fill ? undefined : roomy ? 'roomy' : fillIsRow ? 'row' : 'compact'}
      className={cn(
        'flex w-full min-w-0 cursor-pointer border-dashed transition-colors duration-200',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/40',
        fill
          ? cn(
              'absolute inset-0 @container/dz rounded-none border border-dashed',
              fillIsRow
                ? 'flex-row items-center gap-2 px-3 py-1.5'
                : 'min-h-0 flex-col items-stretch justify-start gap-0 overflow-y-auto p-0',
              isDragActive
                ? 'border-primary bg-primary/5 ring-2 ring-inset ring-primary/20'
                : 'border-muted-foreground/40 bg-muted/15 hover:border-primary/50 hover:bg-muted/25',
            )
          : cn(
              'flex-wrap items-center gap-3',
              collapsed
                ? 'rounded-md border px-2.5 py-1.5'
                : compact
                  ? 'rounded-lg border px-3 py-2'
                  : 'rounded-lg border-2 px-4 py-2.5',
              isDragActive
                ? 'border-primary bg-primary/5 ring-2 ring-primary/20'
                : collapsed || quiet
                  ? 'border-muted-foreground/25 bg-muted/10 hover:border-primary/40 hover:bg-muted/20'
                  : 'border-muted-foreground/25 hover:border-primary/50 hover:bg-muted/30',
            ),
      )}
    >
      <input
        {...getInputProps()}
        aria-label={t('documents.upload.uploadDrop', 'Upload files by clicking or dragging')}
        data-testid="document-dropzone-input"
      />
      {composed ? (
        <div
          className={cn(
            'flex min-h-0 min-w-0 flex-col',
            roomy && 'min-h-0 flex-1',
          )}
        >
          {roomy ? (
            <div className="flex min-h-0 flex-1 flex-col items-center justify-center overflow-y-auto px-6 py-6 text-center">
              <div
                className={cn(
                  'flex h-11 w-11 items-center justify-center rounded-xl border bg-background shadow-xs',
                  isDragActive ? 'border-primary bg-primary/10' : 'border-border',
                )}
              >
                <Upload
                  className={cn('h-5 w-5', isDragActive ? 'text-primary' : 'text-foreground')}
                  aria-hidden
                />
              </div>
              <p
                className={cn(
                  'mt-3 text-base font-medium',
                  isDragActive ? 'text-primary' : 'text-foreground',
                )}
                data-testid="document-dropzone-title"
              >
                {invitationTitle}
              </p>
              <p
                id={hintId}
                className="mt-1 max-w-sm text-sm leading-relaxed text-muted-foreground"
              >
                {invitationHint}
              </p>
              {!isDragActive ? (
                <ChooseFilesButton
                  label={t('documents.upload.chooseFiles', 'Choose files')}
                  onChoose={openFileDialog}
                  className="mt-4"
                />
              ) : null}
              {!isDragActive ? (
                <FormatChips
                  label={t('documents.upload.acceptedFormats', 'Accepted formats')}
                  imagesLabel={imagesLabel}
                />
              ) : null}
              {!isDragActive ? (
                <p className="mt-2 max-w-sm text-xs text-muted-foreground">{limitHint}</p>
              ) : null}
            </div>
          ) : (
            <div
              className={cn(
                'flex w-full min-w-0 gap-3 px-3 py-2.5',
                // Stack = narrow OR short-and-not-wide. From 18rem wide the invitation and
                // its button share one line, so a short tools band is never clipped.
                fillIsStack
                  ? 'flex-col items-stretch @[18rem]/dz:flex-row @[18rem]/dz:items-center'
                  : 'items-center',
              )}
            >
              <div className="flex min-w-0 flex-1 items-center gap-3">
                <div
                  className={cn(
                    'flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border bg-background',
                    isDragActive ? 'border-primary bg-primary/10' : 'border-border',
                  )}
                >
                  <Upload
                    className={cn('h-4 w-4', isDragActive ? 'text-primary' : 'text-foreground')}
                    aria-hidden
                  />
                </div>
                <div className="min-w-0 text-left">
                  <p
                    className={cn(
                      'truncate text-sm font-medium',
                      isDragActive ? 'text-primary' : 'text-foreground',
                    )}
                    data-testid="document-dropzone-title"
                  >
                    {invitationTitle}
                  </p>
                  <p id={hintId} className="truncate text-xs text-muted-foreground">
                    {compactHint}
                  </p>
                </div>
              </div>
              {!isDragActive ? (
                <ChooseFilesButton
                  label={t('documents.upload.chooseFiles', 'Choose files')}
                  onChoose={openFileDialog}
                  className={fillIsStack ? 'w-full @[18rem]/dz:w-auto @[18rem]/dz:shrink-0' : 'shrink-0'}
                />
              ) : null}
            </div>
          )}
          <DropzoneSettingsBar
            stacked={fillIsStack}
            collapseWhenNarrow={!roomy}
            settingsLabel={t('documents.upload.settings', 'Upload settings')}
            label={t('documents.upload.pdfParserFooter', 'PDF parser')}
            hint={t(
              'documents.upload.pdfParserHint',
              'Choose a PDF parser override for this upload, or keep the workspace default.',
            )}
          >
            {settings}
          </DropzoneSettingsBar>
        </div>
      ) : (
        <>
          <div
            className={cn(
              'flex min-w-0 items-center gap-2',
              fillIsRow ? 'min-w-0 flex-1' : 'flex-1 flex-wrap',
            )}
          >
            <div
              className={cn(
                'shrink-0 rounded-lg p-1.5',
                isDragActive ? 'bg-primary/10' : 'bg-muted/50',
              )}
            >
              <Upload
                className={cn(
                  'h-4 w-4',
                  isDragActive ? 'text-primary' : 'text-muted-foreground',
                )}
                aria-hidden
              />
            </div>
            <div className={cn('min-w-0', fillIsRow ? 'flex-1' : 'flex-1 basis-48')}>
              <p
                className={cn(
                  'truncate text-sm font-medium',
                  isDragActive ? 'text-primary' : 'text-foreground',
                )}
                data-testid="document-dropzone-title"
              >
                {fillIsRow && !isDragActive
                  ? t('documents.upload.dropOrChoose', 'Drop files or choose them')
                  : invitationTitle}
              </p>
              <p id={hintId} className="sr-only">
                {invitationHint} {limitHint}
              </p>
            </div>
          </div>
          <div
            className={cn(
              'flex min-w-0 flex-wrap items-center gap-2 overflow-hidden',
              fillIsRow ? 'flex-[3]' : 'shrink-0',
              !fill &&
                'max-sm:basis-full max-sm:border-t max-sm:border-border/50 max-sm:pt-1.5 max-sm:pl-0 sm:border-l sm:border-border/70 sm:pl-3',
            )}
            data-testid="upload-parser-vision-combo"
            onClick={(event) => event.stopPropagation()}
            onKeyDown={(event) => event.stopPropagation()}
          >
            {settings}
          </div>
        </>
      )}
    </div>
  );
}
