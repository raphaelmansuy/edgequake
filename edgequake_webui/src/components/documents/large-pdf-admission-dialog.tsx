'use client';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import type { LargePdfAdmissionPreview, PdfParserChoice } from '@/lib/pdf/large-pdf-admission';
import { estimateIngestMinutes } from '@/lib/pdf/large-pdf-admission';
import { AlertTriangle, FileText } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface LargePdfAdmissionDialogProps {
  open: boolean;
  previews: LargePdfAdmissionPreview[];
  onOpenChange: (open: boolean) => void;
  onConfirm: (parserChoice: PdfParserChoice, files: File[]) => void;
  onCancel: () => void;
}

function formatFileSize(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(bytes / 1024).toFixed(0)} KB`;
}

/**
 * Pre-upload admission card for large PDFs (SPEC-038 REQ-038-04).
 */
export function LargePdfAdmissionDialog({
  open,
  previews,
  onOpenChange,
  onConfirm,
  onCancel,
}: LargePdfAdmissionDialogProps) {
  const { t } = useTranslation();
  const primary = previews[0];
  const [parserChoice, setParserChoice] = useState<PdfParserChoice>('edgeparse');

  const visionMinutes = useMemo(
    () => (primary ? estimateIngestMinutes(primary.pageCount, 'vision') : 0),
    [primary],
  );
  const edgeparseMinutes = useMemo(
    () => (primary ? estimateIngestMinutes(primary.pageCount, 'edgeparse') : 0),
    [primary],
  );

  if (!primary) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="sm:max-w-lg"
        data-testid="spec038-large-pdf-admission-dialog"
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            {t('documents.upload.largePdfTitle', 'Large PDF detected')}
          </DialogTitle>
          <DialogDescription data-testid="spec038-admission-summary">
            {t(
              'documents.upload.largePdfDescription',
              '{{name}} has {{pages}} pages ({{size}}). Choose how to process it.',
              {
                name: primary.file.name,
                pages: primary.pageCount,
                size: formatFileSize(primary.fileSizeBytes),
              },
            )}
          </DialogDescription>
        </DialogHeader>

        <div
          className="rounded-lg border bg-muted/40 p-4 space-y-3"
          data-testid="spec038-admission-recommendation"
        >
          <p className="text-sm font-medium">
            {t(
              'documents.upload.largePdfTextLayer',
              'Born-digital PDFs with embedded text process faster with Fast parse (EdgeParse).',
            )}
          </p>
          <p className="text-sm text-muted-foreground" data-testid="spec038-admission-eta-edgeparse">
            {t('documents.upload.etaEdgeParse', 'Estimated with Fast parse: ~{{minutes}} min', {
              minutes: edgeparseMinutes,
            })}
          </p>
        </div>

        <RadioGroup
          value={parserChoice}
          onValueChange={(v) => setParserChoice(v as PdfParserChoice)}
          className="space-y-2"
          data-testid="spec038-parser-choice"
        >
          <div className="flex items-start gap-2 rounded-md border p-3">
            <RadioGroupItem value="edgeparse" id="spec038-parser-edgeparse" />
            <Label htmlFor="spec038-parser-edgeparse" className="cursor-pointer space-y-1">
              <span className="font-medium">
                {t('documents.upload.parserFastRecommended', 'Fast parse (EdgeParse) — recommended')}
              </span>
              <span className="block text-xs text-muted-foreground">
                ~{edgeparseMinutes} min
              </span>
            </Label>
          </div>
          <div className="flex items-start gap-2 rounded-md border p-3">
            <RadioGroupItem value="vision" id="spec038-parser-vision" />
            <Label htmlFor="spec038-parser-vision" className="cursor-pointer space-y-1">
              <span className="font-medium flex items-center gap-1">
                <AlertTriangle className="h-3.5 w-3.5 text-amber-500" />
                {t('documents.upload.parserVisionSlow', 'Vision OCR (slower)')}
              </span>
              <span
                className="block text-xs text-amber-700 dark:text-amber-400"
                data-testid="spec038-admission-eta-vision"
              >
                {t('documents.upload.etaVisionWarning', 'May take ~{{minutes}} min or fail on timeout', {
                  minutes: visionMinutes,
                })}
              </span>
            </Label>
          </div>
        </RadioGroup>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={onCancel}
            data-testid="spec038-admission-cancel"
          >
            {t('common.cancel', 'Cancel')}
          </Button>
          <Button
            type="button"
            onClick={() =>
              onConfirm(
                parserChoice,
                previews.map((preview) => preview.file),
              )
            }
            data-testid="spec038-admission-confirm"
          >
            {t('documents.upload.uploadAndProcess', 'Upload & Process')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default LargePdfAdmissionDialog;
