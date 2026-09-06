/**
 * SPEC-015V — Vision extract form + upload settings panel (popover).
 *
 * Dropzone stays minimal: Parser + one “Vision” panel trigger.
 * All Vision config (effort, modalities, prompts) lives in the panel form.
 */

'use client';

import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { ReasoningEffortSelect } from '@/components/settings/reasoning-effort-select';
import {
  displayVisionSystemPrompt,
  isCustomVisionSystemPrompt,
  storeVisionSystemPrompt,
  type VisionPromptFieldKey,
} from '@/lib/vision/default-system-prompts';
import { cn } from '@/lib/utils';
import { ChevronDown, Settings2 } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface VisionExtractDraft {
  extractImages: boolean;
  extractCharts: boolean;
  extractFigures: boolean;
  pageSystemPrompt: string;
  imageSystemPrompt: string;
  chartSystemPrompt: string;
  figureSystemPrompt: string;
}

export const DEFAULT_VISION_EXTRACT_DRAFT: VisionExtractDraft = {
  extractImages: true,
  extractCharts: true,
  extractFigures: true,
  pageSystemPrompt: '',
  imageSystemPrompt: '',
  chartSystemPrompt: '',
  figureSystemPrompt: '',
};

const MODALITIES = [
  ['extractImages', 'Images', 'Full-page and drawing crops'] as const,
  ['extractCharts', 'Charts', 'Chart / plot ink crops'] as const,
  ['extractFigures', 'Figures', 'Captioned figure crops'] as const,
];

const PROMPT_FIELDS: readonly {
  key: VisionPromptFieldKey;
  label: string;
}[] = [
  { key: 'pageSystemPrompt', label: 'Page (Pass A)' },
  { key: 'imageSystemPrompt', label: 'Image' },
  { key: 'chartSystemPrompt', label: 'Chart' },
  { key: 'figureSystemPrompt', label: 'Figure' },
];

export function countVisionPromptOverrides(value: VisionExtractDraft): number {
  return PROMPT_FIELDS.filter(({ key }) =>
    isCustomVisionSystemPrompt(key, value[key]),
  ).length;
}

/** Short scent for the panel trigger (defaults → empty / calm). */
export function summarizeVisionExtract(value: VisionExtractDraft): {
  modalityOff: string[];
  promptOverrides: number;
  isDefault: boolean;
} {
  const modalityOff: string[] = [];
  if (!value.extractImages) modalityOff.push('Images');
  if (!value.extractCharts) modalityOff.push('Charts');
  if (!value.extractFigures) modalityOff.push('Figures');
  const promptOverrides = countVisionPromptOverrides(value);
  return {
    modalityOff,
    promptOverrides,
    isDefault: modalityOff.length === 0 && promptOverrides === 0,
  };
}

export interface VisionExtractFormProps {
  value: VisionExtractDraft;
  onChange: (next: VisionExtractDraft) => void;
  showInheritHint?: boolean;
  disabled?: boolean;
  className?: string;
  /** Workspace wizard vs per-upload dropzone (copy + scope). */
  scope?: 'workspace' | 'upload';
  /** Optional effort field rendered at top of the form (upload panel). */
  effort?: {
    value?: string;
    onChange: (value: string | undefined) => void;
    supported?: string[];
    thinkingSupported?: boolean;
    effectiveWhenAuto?: string | null;
  };
}

/**
 * Shared form body — wizard embeds this; upload wraps it in a popover panel.
 * Kept as `VisionExtractControls` for existing imports / testids.
 */
export function VisionExtractControls({
  value,
  onChange,
  showInheritHint = false,
  disabled = false,
  className,
  effort,
  scope = 'upload',
}: VisionExtractFormProps) {
  const { t } = useTranslation();
  const [promptsOpen, setPromptsOpen] = useState(
    () => countVisionPromptOverrides(value) > 0,
  );
  const patch = (partial: Partial<VisionExtractDraft>) =>
    onChange({ ...value, ...partial });
  const promptOverrides = countVisionPromptOverrides(value);

  return (
    <div
      className={cn('space-y-4', className)}
      data-testid="vision-extract-controls"
    >
      {showInheritHint ? (
        <p className="text-xs text-muted-foreground leading-relaxed">
          {t(
            'documents.upload.visionExtractInheritHint',
            'Defaults follow the workspace; changes apply to this upload only.',
          )}
        </p>
      ) : null}

      {effort ? (
        <div className="space-y-1.5" data-testid="pdf-vision-reasoning-effort">
          <Label className="text-xs font-medium text-muted-foreground">
            {t('documents.upload.visionReasoningEffort', 'Vision effort')}
          </Label>
          <ReasoningEffortSelect
            value={effort.value}
            onChange={effort.onChange}
            supported={effort.supported}
            thinkingSupported={effort.thinkingSupported}
            effectiveWhenAuto={effort.effectiveWhenAuto}
            hideHint
            compactTrigger
            className="w-full"
            triggerClassName="h-9 w-full"
            label={t('documents.upload.visionReasoningEffort', 'Vision effort')}
            data-testid="pdf-vision-reasoning-effort-select"
            disabled={disabled}
          />
        </div>
      ) : null}

      <div className="space-y-2">
        <div>
          <p className="text-xs font-medium text-muted-foreground">
            {t('documents.upload.visionExtract.modalities', 'Extract')}
          </p>
          <p className="text-[11px] text-muted-foreground mt-0.5">
            {t(
              scope === 'workspace'
                ? 'onboarding.visionExtractModalitiesHint'
                : 'documents.upload.visionExtract.modalitiesHint',
              scope === 'workspace'
                ? 'Turn off modalities you do not need. Applies to future uploads.'
                : 'Turn off modalities you do not need for this upload.',
            )}
          </p>
        </div>
        <div
          className="divide-y rounded-md border"
          role="group"
          aria-label={t(
            'documents.upload.visionExtract.groupLabel',
            'Vision extraction modalities',
          )}
        >
          {MODALITIES.map(([key, label, hint]) => (
            <label
              key={key}
              htmlFor={`vision-${key}`}
              className="flex items-center justify-between gap-3 px-3 py-2.5 cursor-pointer select-none"
            >
              <span className="min-w-0">
                <span className="block text-sm font-medium leading-none">
                  {t(`documents.upload.visionExtract.${key}`, label)}
                </span>
                <span className="block text-[11px] text-muted-foreground mt-1 leading-snug">
                  {t(`documents.upload.visionExtract.hint.${key}`, hint)}
                </span>
              </span>
              <Switch
                id={`vision-${key}`}
                checked={value[key]}
                disabled={disabled}
                data-testid={`vision-extract-${key}`}
                onCheckedChange={(v) => patch({ [key]: v })}
              />
            </label>
          ))}
        </div>
      </div>

      <div className="space-y-2">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-auto px-0 text-xs text-muted-foreground hover:text-foreground"
          disabled={disabled}
          data-testid="vision-extract-advanced-toggle"
          aria-expanded={promptsOpen}
          onClick={() => setPromptsOpen((o) => !o)}
        >
          {promptsOpen
            ? t(
                'documents.upload.visionExtract.hidePrompts',
                'Hide extraction prompts',
              )
            : t(
                'documents.upload.visionExtract.showPrompts',
                'Extraction prompts (advanced)',
              )}
          {promptOverrides > 0 && !promptsOpen ? (
            <span className="ml-1 tabular-nums">({promptOverrides})</span>
          ) : null}
        </Button>

        {promptsOpen ? (
          <div className="space-y-3" data-testid="vision-extract-prompts">
            <p className="text-[11px] text-muted-foreground leading-relaxed">
              {t(
                'documents.upload.visionExtract.promptIntro',
                'What you see is the system prompt Vision uses. Edit to override; Reset restores the built-in default.',
              )}
            </p>
            {PROMPT_FIELDS.map(({ key, label }) => {
              const custom = isCustomVisionSystemPrompt(key, value[key]);
              return (
                <div key={key} className="space-y-1">
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2 min-w-0">
                      <Label className="text-xs font-medium">
                        {t(`documents.upload.visionExtract.prompt.${key}`, label)}
                      </Label>
                      <span
                        className={cn(
                          'text-[10px] uppercase tracking-wide shrink-0',
                          custom
                            ? 'text-foreground'
                            : 'text-muted-foreground',
                        )}
                        data-testid={`vision-extract-prompt-mode-${key}`}
                      >
                        {custom
                          ? t('documents.upload.visionExtract.promptCustom', 'Custom')
                          : t(
                              'documents.upload.visionExtract.promptBuiltIn',
                              'Built-in',
                            )}
                      </span>
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="h-auto px-1 text-xs"
                      disabled={disabled || !custom}
                      onClick={() => patch({ [key]: '' })}
                    >
                      {t('documents.upload.visionExtract.resetPrompt', 'Reset')}
                    </Button>
                  </div>
                  <Textarea
                    value={displayVisionSystemPrompt(key, value[key])}
                    disabled={disabled}
                    rows={6}
                    className={cn(
                      'font-mono text-xs min-h-[7rem]',
                      !custom && 'text-muted-foreground',
                    )}
                    data-testid={`vision-extract-prompt-${key}`}
                    onChange={(e) =>
                      patch({ [key]: storeVisionSystemPrompt(key, e.target.value) })
                    }
                    onFocus={(e) => {
                      // Select-all on first focus of built-in so typing replaces cleanly.
                      if (!custom) {
                        e.currentTarget.select();
                      }
                    }}
                  />
                </div>
              );
            })}
          </div>
        ) : null}
      </div>
    </div>
  );
}

export interface VisionSettingsPanelProps {
  value: VisionExtractDraft;
  onChange: (next: VisionExtractDraft) => void;
  showInheritHint?: boolean;
  disabled?: boolean;
  compact?: boolean;
  effort?: VisionExtractFormProps['effort'];
  className?: string;
}

/**
 * Upload toolbar: one calm trigger → full Vision config form in a panel.
 */
export function VisionSettingsPanel({
  value,
  onChange,
  showInheritHint = false,
  disabled = false,
  compact = false,
  effort,
  className,
}: VisionSettingsPanelProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const summary = useMemo(() => summarizeVisionExtract(value), [value]);
  const effortNonDefault = Boolean(effort?.value);

  const scent = useMemo(() => {
    const parts: string[] = [];
    if (summary.modalityOff.length > 0) {
      parts.push(
        t('documents.upload.visionExtract.offSummary', 'Off: {{list}}', {
          list: summary.modalityOff.join(', '),
        }),
      );
    }
    if (summary.promptOverrides > 0) {
      parts.push(
        t('documents.upload.visionExtract.promptSummary', '{{count}} prompts', {
          count: summary.promptOverrides,
        }),
      );
    }
    if (effortNonDefault) {
      parts.push(
        t('documents.upload.visionExtract.effortSet', 'Effort set'),
      );
    }
    return parts.join(' · ');
  }, [effortNonDefault, summary, t]);

  const customized = !summary.isDefault || effortNonDefault;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="outline"
          disabled={disabled}
          data-testid="vision-settings-panel-trigger"
          aria-expanded={open}
          title={
            scent ||
            t(
              'documents.upload.visionSettingsTitle',
              'Vision extraction settings for this upload',
            )
          }
          className={cn(
            'bg-background justify-between gap-1.5 font-normal min-w-0 max-w-full',
            compact ? 'h-7 px-2 text-xs' : 'h-9 px-3 text-sm',
            customized && 'border-foreground/30',
            className,
          )}
          onClick={(e) => e.stopPropagation()}
        >
          <span className="inline-flex items-center gap-1.5 min-w-0">
            <Settings2 className={cn('shrink-0 opacity-70', compact ? 'size-3' : 'size-3.5')} />
            <span className="truncate">
              {t('documents.upload.visionSettings', 'Vision')}
            </span>
            {customized ? (
              <span
                className="size-1.5 rounded-full bg-foreground shrink-0"
                aria-hidden
              />
            ) : null}
          </span>
          <ChevronDown
            className={cn(
              'shrink-0 opacity-50 transition-transform',
              compact ? 'size-3' : 'size-3.5',
              open && 'rotate-180',
            )}
          />
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        side="bottom"
        sideOffset={6}
        className="w-[22rem] max-h-[min(75vh,32rem)] overflow-y-auto p-4"
        data-testid="vision-settings-panel"
        onClick={(e) => e.stopPropagation()}
        onOpenAutoFocus={(e) => e.preventDefault()}
      >
        <div className="space-y-3">
          <div className="space-y-0.5">
            <h4 className="text-sm font-medium leading-none">
              {t('documents.upload.visionSettingsHeading', 'Vision settings')}
            </h4>
            <p className="text-[11px] text-muted-foreground leading-relaxed">
              {t(
                'documents.upload.visionSettingsSubheading',
                'Applies to this upload only. Workspace defaults stay unchanged.',
              )}
            </p>
            {scent ? (
              <p className="text-[11px] text-foreground/80 pt-1">{scent}</p>
            ) : null}
          </div>
          <VisionExtractControls
            value={value}
            onChange={onChange}
            showInheritHint={showInheritHint}
            disabled={disabled}
            effort={effort}
          />
        </div>
      </PopoverContent>
    </Popover>
  );
}

/** Whether resolved parser choice shows Vision extract controls. */
export function shouldShowVisionExtractControls(
  parserChoice: 'none' | 'default' | 'vision' | 'edgeparse' | 'auto',
  serverOrWorkspaceIsVision: boolean,
): boolean {
  if (parserChoice === 'edgeparse') return false;
  if (parserChoice === 'vision' || parserChoice === 'auto') return true;
  return serverOrWorkspaceIsVision;
}
